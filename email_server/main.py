import asyncio
import httpx
import smtplib
import aiosqlite
from fastapi import FastAPI
from email.message import EmailMessage
from pydantic import BaseModel, EmailStr
from typing import Optional
from datetime import datetime
import uvicorn

app = FastAPI()

MAILTM_API = "https://api.mail.tm"
SMTP_SERVER = "smtp.gmail.com"
SMTP_PORT = 587
SMTP_USERNAME = "your@gmail.com"
SMTP_PASSWORD = "your_password"

DB_PATH = "email_map.db"

# ========== DB INIT ==========

async def init_db():
    async with aiosqlite.connect(DB_PATH) as db:
        await db.execute("""
            CREATE TABLE IF NOT EXISTS email_map (
                real_email TEXT PRIMARY KEY,
                proxy_email TEXT,
                proxy_id TEXT,
                token TEXT,
                expire_date TEXT
            )
        """)
        await db.commit()

@app.on_event("startup")
async def startup_event():
    await init_db()
    asyncio.create_task(poll_mailtm())

# ========== MODELS ==========

class RegisterRequest(BaseModel):
    real_email: EmailStr
    expire_date: datetime  # ISO8601 format

class RegisterResponse(BaseModel):
    proxy_email: str

# ========== HELPERS ==========

async def mailtm_create_account() -> dict:
    async with httpx.AsyncClient() as client:
        domain = (await client.get(f"{MAILTM_API}/domains")).json()["hydra:member"][0]["domain"]
        username = f"user{str(hash(asyncio.get_event_loop().time()))[:8]}"
        email = f"{username}@{domain}"
        password = "SuperSecure123!"

        await client.post(f"{MAILTM_API}/accounts", json={
            "address": email,
            "password": password
        })

        token_resp = await client.post(f"{MAILTM_API}/token", json={
            "address": email,
            "password": password
        })

        token = token_resp.json()["token"]

        # Get account ID
        headers = {"Authorization": f"Bearer {token}"}
        account = (await client.get(f"{MAILTM_API}/me", headers=headers)).json()
        return {
            "email": email,
            "id": account["id"],
            "token": token
        }

async def forward_email(to_email: str, subject: str, content: str):
    msg = EmailMessage()
    msg["Subject"] = subject
    msg["From"] = "forwarder@instantdmv.xyz"
    msg["To"] = to_email
    msg.set_content(content)

    try:
        with smtplib.SMTP(SMTP_SERVER, SMTP_PORT) as server:
            server.starttls()
            server.login(SMTP_USERNAME, SMTP_PASSWORD)
            server.send_message(msg)
        print(f"Forwarded to {to_email}")
    except Exception as e:
        print(f"Failed to forward: {e}")

# ========== ROUTES ==========

@app.post("/register", response_model=RegisterResponse)
async def register_forwarder(req: RegisterRequest):
    async with aiosqlite.connect(DB_PATH) as db:
        cursor = await db.execute("SELECT proxy_email FROM email_map WHERE real_email = ?", (req.real_email,))
        row = await cursor.fetchone()
        if row:
            return {"proxy_email": row[0]}

        acct = await mailtm_create_account()
        await db.execute(
            "INSERT INTO email_map (real_email, proxy_email, proxy_id, token, expire_date) VALUES (?, ?, ?, ?, ?)",
            (req.real_email, acct["email"], acct["id"], acct["token"], req.expire_date.isoformat())
        )
        await db.commit()
        return {"proxy_email": acct["email"]}

# ========== BACKGROUND POLLER ==========

async def poll_mailtm():
    while True:
        try:
            async with aiosqlite.connect(DB_PATH) as db:
                cursor = await db.execute("SELECT real_email, proxy_email, proxy_id, token, expire_date FROM email_map")
                rows = await cursor.fetchall()

                for real_email, proxy_email, proxy_id, token, expire_date_str in rows:
                    try:
                        expire_date = datetime.fromisoformat(expire_date_str)
                        if datetime.utcnow() > expire_date:
                            continue  # skip expired
                    except Exception as e:
                        print(f"Invalid date for {real_email}: {e}")
                        continue

                    headers = {"Authorization": f"Bearer {token}"}
                    async with httpx.AsyncClient() as client:
                        messages = (await client.get(f"{MAILTM_API}/messages", headers=headers)).json().get("hydra:member", [])

                        for msg in messages:
                            detail = (await client.get(f"{MAILTM_API}/messages/{msg['id']}", headers=headers)).json()
                            subject = detail.get("subject", "(no subject)")
                            content = detail.get("text", "(empty)")

                            await forward_email(real_email, subject, content)
                            await client.delete(f"{MAILTM_API}/messages/{msg['id']}", headers=headers)

        except Exception as e:
            print(f"Polling error: {e}")

        await asyncio.sleep(15)

# ========== ENTRY ==========

if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=8000)
