# User registration using (and only using) passkey

## Background
Currently, users can post to this system after authenticate the validaty using captcha.
After the authentication, user agent has cookie authed token which is necesarry to check the validity of the user when they want to post.

But, this token is device bound, thus we need to fix to be not device bounded.

## User model
Passkey user account (i.e., user account) is intended to indicate only one user.
User account able to have many passkeys, authed tokens.
User can have only one ID on a board day by day, and this should not affected by multiple authed tokens.
In most cases, authed tokens does not connect to user account.

When user want to register passkey at first time, eddist create passkey user automatically. This user account has randomly generated user name which user can change later.

## Implementation
We use passkey as simple user registration system (and can avoid some abusing).
There are some registration method,
1. Use passkey as initial registration, then create authed token (use in mail ONLY ONCE)
2. Use auth-code as initial registration. After authentication, user register own passkey
3. Already registrated some method, and user register passkey to associate authed token

and they have priority, we implement 3 at first, and 2 to 1.

### 1: Use passkey as initial registration
```mermaid
sequenceDiagram
    actor U as User
    participant B as Browser
    participant E as Eddist
    participant R as Redis
    participant DB as MySQL
    U ->> B: request to initial registration
    B ->> E: access `/user/registration`
    E ->> B: return HTML w/ assets
    B ->> E: fetch `/api/user/registration`
    E ->> R: generate challenge and save auth info <br/> User names is generated randomly
    E ->> B: send challenge
    B ->> B: navigator.credentials.create() <br/> (Authenticator process)
    B ->> E: Send authentication data to eddist
    E ->> R: get auth data from Redis
    R ->> E: auth data
    E ->> E: validate authentication response
    E ->> DB: Create authed token and passkey authed user
    E ->> B: send authed token and link to login page
    B ->> U: Sign-up finished
```

### 2: Use auth-code as initial registration
```mermaid
sequenceDiagram
    actor U as User
    participant S as Specialized Browser
    participant B as Web Browser
    participant E as Eddist
    participant R as Redis
    participant DB as MySQL
    U ->> S: Write response as new visitor
    S ->> E: Send creating response request
    E ->> DB: Create authed token w/ code
    E ->> S: authentication request w/ auth code
    S ->> U: show auth code w/ authentication url <br/> (normal url / passkey url)
    U ->> B: open passkey url
    B ->> E: request to passkey url
    E ->> B: return assets
    B ->> B: some validation to prevent abuser
    B ->> U: show passkey authentication page
    U ->> B: input auth code
    B ->> E: send auth code and validation result
    E ->> E: check auth code, validation result <br/> generate challenge, registration info
    E ->> R: save challeng, registration info
    E ->> B: send request and registration info
    B ->> B: navigator.credentials.create() <br/> (Authenticator process)
    B ->> E: Send authentication result
    E ->> R: get authentication data
    R ->> E: auth data
    E ->> E: validate authentication data
    E ->> DB: Create passkey user and update authed token?
    E ->> B: Send authed success page w/ authed token
    B ->> U: Sign-up complete
```

### 3: Already registered using some method
```mermaid
sequenceDiagram
    actor U as User
    participant S as Specialized Browser
    participant B as Web Browser
    participant E as Eddist
    participant R as Redis
    participant DB as MySQL
    U ->> S: Request passkey registration <br/> using some command
    S ->> E: Send response
    E ->> E: generate challenge, authentication data
    E ->> R: save challenge and auth data
    E ->> S: issue temporary code (5 minutes and can use once) w/ passkey registration url
    S ->> U: show tmp code and passkey registration url
    U ->> B: open passkey registration url
    B ->> E: request to passkey registration url
    E ->> B: return assets
    B ->> U: show page and prompt to input tmp code
    U ->> B: input tmp code
    B ->> E: send tmp code
    E ->> R: get authed token, challenge
    R ->> E: authed token, challenge, some auth info
    E ->> B: send challenge, some auth data
    B ->> B: navigator.credentials.create() <br/> (Authenticator process)
    B ->> E: send authentication result
    E ->> R: get some auth data
    R ->> E: some auth data
    E ->> E: validate auth data
    E ->> DB: Create passkey user and update authed token?
    E ->> B: Send authentication success page w/ login page link
    B ->> U: Passkey registration completed 
```

## Need components for passkey implementation

### Core components
- Login page
- Management page
    - management passkey
    - management authed token
        - should be able to register plan3 using tmp code
- Passkey registration page
    - Plan 1~3

### Additional components
TODO
