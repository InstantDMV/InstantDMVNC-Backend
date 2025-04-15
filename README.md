# InstantDMV North Carolina

### Disclaimer
This is meant to only get 1 appointment on behalf of a client, in no way are we "hogging" or scalping these to resell, we are placing an appointment on a clients schedule. We do not condone scalping of any service.

### Intro
I (@ElijahBare) have started work on this project due to my troubles with getting an appointment at the DMV in NC
it is a painpoint for everyone, especially with the RealID deadline approaching.

## Features
- scans appointments with thirtyfour (selenium bindings for rust)
- able to book appointment for you
- solves captcha
- more than just 18+ new driver license appointments, user should able to choose (Done)
- Filters by distance with zipcode.

## TODO
- better error handling in the selenium instance
- automation of downloading/installing chromedriver
- cli for open source users

### Sidenote on the stack
I know i am killing an ant with a bazooka given my stack and the application but I wanted to work on my rust abilities and how to write better rust code so I picked the highly performant 'actix-web' library. It wasnt neccesary for this application but i decided to use it anyway + this will make hosting cheaper if i make this a service in the end.
