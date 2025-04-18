import asyncio
import logging
import os
import re
from datetime import datetime, timezone

from fastapi import FastAPI, HTTPException
from motor.motor_asyncio import AsyncIOMotorClient
from pydantic import BaseModel, EmailStr
import httpx
from bs4 import BeautifulSoup
from dotenv import load_dotenv
import uvicorn

# ─── Configuration ─────────────────────────────────────────────────────────────
load_dotenv()

MAILTM_API = os.getenv("MAILTM_API", "https://api.mail.tm")
POLL_INTERVAL = int(os.getenv("POLL_INTERVAL", 15))

MONGODB_URI = os.getenv("MONGODB_URI")
MONGODB_DB = os.getenv("MONGODB_DB", "InstantDMV")
MONGODB_COLLECTION = os.getenv("MONGODB_COLLECTION", "email_map")

# ─── Logging Setup ─────────────────────────────────────────────────────────────
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s"
)
logger = logging.getLogger("email_link_clicker")

# ─── FastAPI + Database Initialization ────────────────────────────────────────
app = FastAPI()
client = AsyncIOMotorClient(MONGODB_URI)
db = client["InstantDMV"]
email_map = db["mailusers"]

class RegisterRequest(BaseModel):
    real_email: EmailStr
    expire_date: datetime

class RegisterResponse(BaseModel):
    proxy_email: str

@app.on_event("startup")
async def startup_event():
    await email_map.create_index("real_email", unique=True)
    app.state.http = httpx.AsyncClient(base_url=MAILTM_API)
    asyncio.create_task(poll_mailtm())

@app.on_event("shutdown")
async def shutdown_event():
    await app.state.http.aclose()

# ─── Register Endpoint ────────────────────────────────────────────────────────
@app.post("/register", response_model=RegisterResponse)
async def register(req: RegisterRequest):
    existing = await email_map.find_one({"real_email": req.real_email})
    if existing:
        return {"proxy_email": existing["proxy_email"]}

    # Create Mail.tm account
    doms = (await app.state.http.get("/domains")).json()["hydra:member"]
    domain = doms[0]["domain"]
    username = f"user{int(datetime.utcnow().timestamp())}"
    password = os.getenv("MAILTM_DEFAULT_PASS", "SuperSecure123!")

    await app.state.http.post("/accounts", json={"address": f"{username}@{domain}", "password": password})
    token = (await app.state.http.post("/token", json={"address": f"{username}@{domain}", "password": password})).json()["token"]

    await email_map.insert_one({
        "real_email": req.real_email,
        "proxy_email": f"{username}@{domain}",
        "proxy_id": username,
        "token": token,
        "expire_date": req.expire_date.isoformat()
    })
    logger.info("Registered %s → %s", req.real_email, f"{username}@{domain}")
    return {"proxy_email": f"{username}@{domain}"}

# ─── Link-Clicker Helper ───────────────────────────────────────────────────────
async def click_links_in_email(detail: dict):
    links = set()

    # Extract URLs from plain text
    text = detail.get("text", "")
    links.update(re.findall(r'https?://\S+', text))

    # Extract from HTML
    html = detail.get("html")
    if isinstance(html, list):
        html = "\n".join(html)
    if html:
        soup = BeautifulSoup(html, "html.parser")
        for a in soup.find_all("a", href=True):
            href = a["href"]
            if href.startswith("http"):
                links.add(href)

    logger.info("Found %d links to click", len(links))
    for link in links:

        try:
            logger.info("Clicking link: %s", link)
            await app.state.http.get(link)
        except Exception as e:
            logger.warning("Error clicking %s: %s", link, e)

# ─── Background Poller ────────────────────────────────────────────────────────
async def poll_mailtm():
    http = app.state.http
    while True:
        try:
            cursor = email_map.find({})
            async for doc in cursor:
                # Skip expired accounts
                expire_date = datetime.fromisoformat(doc["expire_date"])
                if datetime.now(timezone.utc) > expire_date:
                    continue  # Skip expired emails

                headers = {"Authorization": f"Bearer {doc['token']}"}
                msgs = (await http.get("/messages", headers=headers)).json().get("hydra:member", [])
                for m in msgs:
                    detail = (await http.get(f"/messages/{m['id']}", headers=headers)).json()
                    await click_links_in_email(detail)

        except Exception as e:
            logger.exception("Polling error: %s", e)

        await asyncio.sleep(POLL_INTERVAL)

# ─── Run Server ───────────────────────────────────────────────────────────────
if __name__ == "__main__":
    uvicorn.run("main:app", host="0.0.0.0", port=8000, log_level="info")
