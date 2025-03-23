# Abstract
This document details the design of the user registration system for Eddist. 
Registered users authenticate via third-party IdPs (OAuth 2.0/OpenID Connect) without local passwords. 
A secure registration flow is triggered by a message command, leveraging cryptographically generated temporary URLs and state parameters. 
The system binds authed tokens to user accounts and manages user sessions for message posting, ensuring robust user identity management.

# Background
Currently, the user need to auth to post a message. But auth system which only depends on the token is not enough, because below reasons:
- The user is hard to remember the token, so the user need to memorize the token or save the token in some place to use the token another device.
- We want to revoke the token which is used for abuse, so user registration system is beneficial to distinguish the user is abuser or not.

# Specification
## Abstract
- User registration system is not required to post a message currently
- We will provide some features only for registered user
- User registration system use only third-party IdP to authenticate the user
  - OAuth 2.0 / OpenID Connect compatible IdP is supported
  - We does not store the user's password because of this choice
  - We support multiple IdP to authenticate the user
    - User can login one of the IdP which is supported by Eddist and registered by the user
- User auth first, then the user can register using user registration process
  - User registration process can begin from writing a message including the some specific command
- We will add flag whether enable this feature or not for while
  - We will create beta env in the future, and test it, then we will enable it in the production env

## Sequence Diagram of User Registration / Management
### Registration Flow
```mermaid
sequenceDiagram
    participant User
    participant Eddist
    participant Redis
    participant DB
    participant IdP

    Note right of User: User already authed
    User->>Eddist: Write a message w/ user registration command
    Eddist->>DB: Check authed token is valid
    alt Invalid
        Eddist->>User: Show error message
        Note right of User: End of the flow
        DB->>Eddist: Return the authed token status
    else Valid
        Eddist->>DB: Check whether the authed token is bound to the user (registered user)
        alt Registered
            DB->>Eddist: Return the user's information
            Eddist->>User: Show the error message (already registered)
            Note right of User: End of the flow
        else Not registered
            Eddist->>Eddist: Create a temporary URL to register the user
            Eddist->>Redis: Store the temporary URL <br> (expiration: 3 minutes, only one time, prefix: userreg:tempurl:register)
            Eddist->>User: Show temporary URL to register the user
        end
    end
    
    User->>Eddist: Access the temporary URL
    Eddist->>Redis: Check user-sid if available
    opt User has user-sid and user-sid is bound to the valid user
        Eddist->>Redis: Delete the temporary URL <br> (expire userreg:tempurl:register)
        Eddist->>User: Redirect to User page
        Note right of User: End of the flow
    end 
    Eddist->>Redis: Retrieve the authed token using the temporary URL and expire the temporary URL <br> (expire userreg:tempurl:register)
    Redis->>Eddist: Return the authed token
    Eddist->>Redis: Generate a state / session ID containing the authed token and some information, then store it <br> (expiration: 3min, prefix: userreg:oauth2:state)
    Eddist->>User: Show the confirmation page to register the user and redirect to IdP
    User->>Eddist: Confirm the registration and redirect to IdP
    Eddist->>Eddist: Generate authorization request parameters (state, nonce, PKCE)    
    Eddist->>Redis: Store the authorization request parameters <br> (expiration: 15min, key: state, prefix: userreg:oauth2:authreq, expire userreg:oauth2:state)
    Eddist->>User: Redirect to IdP (authorization endpoint)
    User->>IdP: Access the authorization endpoint
    IdP<<->>User: Authenticate & Authorize in IdP
    IdP->>Eddist: Redirect to Eddist (callback URL w/ authorization code, state)
    Eddist->>Redis: Retrieve the authorization request parameters, authed token using the state and expire the state <br> (expire userreg:oauth2:authreq)
    Eddist->>IdP: Get access token
    Redis->>Eddist: Return the authorization request parameters, authed token and some information
    Eddist->>DB: Save the user's information and bind the user to the authed token
    Eddist->>Eddist: Create user session
    Eddist->>Redis: Store the user session <br> (expiration: 365 days, prefix: user:session)
    Eddist->>User: Show the user page w/ success message and user session
```

### Login flow
```mermaid
sequenceDiagram
    participant User
    participant Eddist
    participant Redis
    participant DB
    participant IdP

    User->>Eddist: Access the login page
    Eddist->>User: Show the login page
    User->>Eddist: Click the IdP button
    Eddist->>Eddist: Generate a state / session ID containing some information, then store it <br> (expiration: 15min, prefix: userlogin:oauth2:authreq)
    Eddist->>User: Redirect to IdP (authorization endpoint)
    User->>IdP: Access the authorization endpoint
    IdP<<->>User: Authenticate & Authorize in IdP
    IdP->>Eddist: Redirect to Eddist (callback URL w/ authorization code, state)
    Eddist->>Redis: Retrieve the authorization request parameters using the state and expire the authreq <br> (expire userlogin:oauth2:authreq)
    Eddist->>IdP: Get access token
    Redis->>Eddist: Return the authorization request parameters and some information
    Eddist->>DB: Check the user is registered or not
    alt Registered
        DB->>Eddist: Return the user's information
        Eddist->>Eddist: Create user session
        Eddist->>Redis: Store the user session <br> (expiration: 365 days, prefix: user:session)
        Eddist->>User: Show the user page w/ success message and user session
    else Not registered
        Eddist->>User: Show the error message (not registered)
    end
```

### Authentication w/ Login flow
```mermaid
sequenceDiagram
    participant User
    participant Eddist
    participant Redis
    participant DB
    participant IdP

    Note right of User: User is not authed
    User->>Eddist: Attempt to write a message
    Eddist->>User: Show the error message w/ temporary auth code
    User->>Eddist: Access the login page
    Note right of User: After successful of login flow
    Eddist->>User: Show the user page
    User->>Eddist: Input the temporary auth code
    Eddist->>DB: Check the temporary auth code is valid
    alt Invalid
        Eddist->>User: Show the error message
        Note right of User: End of the flow
    else Valid
        Eddist->>DB: Activate the authed token and bind the authed token to the user
        Eddist->>User: Show the user page w/ success message
    end
```

### Post flow
TODO: need to further discussion
```mermaid
sequenceDiagram
    participant User
    participant Eddist
    participant Redis
    participant DB

    Note right of User: User already authed
    User->>Eddist: Write a message
    Eddist->>DB: Check authed token is valid
    alt Invalid
        Eddist->>User: Show error message
        Note right of User: End of the flow
        DB->>Eddist: Return the authed token status
    else Valid
        alt User has user session
            Eddist->>Redis: Retrieve the user session using user session
            Redis->>Eddist: Return the user session
            Eddist->>DB: Check the user session (user status) is valid
            alt Invalid (not found)
                Eddist->>User: Remove the user session cookie
                Note right of User: Continue with normal post flow
            else Invalid (abuser)
                Eddist->>Redis: Expire the user session
                Eddist->>DB: Revoke the authed token
                Eddist->>User: Show the error message
            else Valid
                Eddist->>DB: Get all of authed tokens which are bound to the user
                DB->>Eddist: Return the authed tokens
                alt Current authed token is included in the authed tokens
                    Note right of User: Continue with normal post flow
                else Current authed token is not included in the authed tokens
                    Eddist->>DB: Check the authed token is bound to the another user
                    alt is bound to the another user
                        Eddist->>DB: Revoke the authed token
                        Eddist->>User: Show the error message
                    else is not bound to the another user
                        Eddist->>DB: Bind the authed token to the user
                        Note right of User: Continue with normal post flow
                    end
                end
            end
        else User does not have user session
            Note right of User: This is not user flow (normal post flow)
        end
    end
```

## ER Diagram associated with User Registration / Management
```mermaid
erDiagram
    %% authed_tokens is already defined, so we does not write the definition here
    authed_tokens {
        %% ony added fields
        boolean require_user_registration
        string registered_user_id
    }
    users {
        string id
        %% user_name is optional, user can set the user name after registration
        string user_name
        string created_at
        string updated_at
    }
    user_authed_tokens {
        string id
        string user_id
        string authed_token_id
        string created_at
        string updated_at
    }
    user_idp_bindings {
        string id
        string user_id
        string idp_id
        string idp_sub
        string created_at
        string updated_at
    }
    idps {
        string id
        string idp_name
        string oidc_config_url
        string client_id
        %% client_secret is encrypted using ChaCha20-Poly1305
        string client_secret
        boolean enabled
    }

    users ||--o{ user_authed_tokens: user_id
    users ||--o{ user_idp_bindings: user_id    
    authed_tokens ||--o{ user_authed_tokens: authed_token
    idps ||--o{ user_idp_bindings: idp_id
```

## URL / API Endpoints
- `/user/register/{tempurl}`: Show the user registration page
- `/user/register/authz/idp/{idp_name}`: Generate the authorization request parameters and redirect to IdP
- `/user/auth/callback`: Callback URL for IdP
- `/user/`: User page
- `/user/login`: Login page
- `/user/login/authz/idp/{idp_name}`: Generate the authorization request parameters and redirect to IdP
- `/user/logout`: Logout page (if possible)
