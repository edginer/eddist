#!/usr/bin/env python3
"""Backfill user_idp_bindings.idp_sub from plaintext IdP `sub` claims to
SHA-256(sub || issuer), matching hash_idp_sub() in
eddist-server/src/services/user_authz_idp_callback_service.rs.

Usage:
    pip install pymysql requests
    python3 scripts/backfill_hash_idp_sub.py

By default DATABASE_URL is read from .env (falling back to
.docker-compose.env) in the repo root; pass --database-url or --env-file to
override.

Run once with no --apply first to see what would change (dry run is the
default). Pass --apply to actually write. Safe to re-run: rows whose
idp_sub already looks like a 64-char lowercase hex hash are skipped, since
no real IdP `sub` claim takes that exact shape.

The issuer used for hashing is read from each IdP's own OIDC discovery
document (idps.oidc_config_url -> the `issuer` field), not guessed from the
URL: OIDC Discovery requires that field to be byte-identical to the `iss`
claim in tokens issued by that provider, which is exactly what
id_token_claims.issuer() reads at login time in the Rust code.
"""

import argparse
import hashlib
import re
import sys
import uuid
from pathlib import Path
from urllib.parse import unquote, urlsplit

try:
    import pymysql
    import requests
except ImportError as e:
    sys.exit(f"missing dependency: pip install pymysql requests ({e})")

HEX64_RE = re.compile(r"^[0-9a-f]{64}$")
REPO_ROOT = Path(__file__).resolve().parent.parent


def load_database_url_from_env_file(env_file: Path | None) -> str | None:
    candidates = [env_file] if env_file else [REPO_ROOT / ".env", REPO_ROOT / ".docker-compose.env"]
    for path in candidates:
        if path is None or not path.is_file():
            continue
        for line in path.read_text().splitlines():
            line = line.strip()
            if not line or line.startswith("#") or "=" not in line:
                continue
            key, _, value = line.partition("=")
            if key.strip() == "DATABASE_URL":
                return value.strip().strip('"').strip("'")
    return None


def parse_database_url(url: str) -> dict:
    parts = urlsplit(url)
    if parts.scheme != "mysql" or not parts.hostname or not parts.path.lstrip("/"):
        sys.exit(f"could not parse --database-url: {url!r}")
    return dict(
        user=unquote(parts.username) if parts.username else None,
        password=unquote(parts.password) if parts.password else "",
        host=parts.hostname,
        port=parts.port or 3306,
        database=parts.path.lstrip("/"),
    )


def hash_idp_sub(sub: str, issuer: str) -> str:
    return hashlib.sha256((sub + "|" + issuer).encode()).hexdigest()


def fetch_issuer(oidc_config_url: str) -> str:
    resp = requests.get(oidc_config_url, timeout=10)
    resp.raise_for_status()
    issuer = resp.json().get("issuer")
    if not issuer:
        sys.exit(f"discovery document at {oidc_config_url} has no 'issuer' field")
    return issuer


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--database-url",
        help="defaults to DATABASE_URL from .env / .docker-compose.env in the repo root",
    )
    ap.add_argument(
        "--env-file",
        type=Path,
        help="explicit .env-style file to read DATABASE_URL from",
    )
    ap.add_argument("--apply", action="store_true", help="write changes (default: dry run)")
    args = ap.parse_args()

    database_url = args.database_url or load_database_url_from_env_file(args.env_file)
    if not database_url:
        sys.exit(
            "no --database-url given and DATABASE_URL not found in .env / .docker-compose.env "
            "(pass --database-url or --env-file explicitly)"
        )

    conn = pymysql.connect(**parse_database_url(database_url), autocommit=False)
    try:
        with conn.cursor() as cur:
            cur.execute("SELECT idp_name, oidc_config_url FROM idps")
            idps = cur.fetchall()

        issuer_by_idp_name = {}
        for idp_name, oidc_config_url in idps:
            issuer = fetch_issuer(oidc_config_url)
            print(f"idp {idp_name!r}: issuer = {issuer!r} (from {oidc_config_url})")
            issuer_by_idp_name[idp_name] = issuer

        total = updated = skipped_already_hashed = 0
        with conn.cursor() as cur:
            cur.execute(
                """
                SELECT uib.id, uib.idp_sub, idps.idp_name
                FROM user_idp_bindings uib
                JOIN idps ON uib.idp_id = idps.id
                """
            )
            rows = cur.fetchall()

        for row_id, idp_sub, idp_name in rows:
            total += 1
            if HEX64_RE.match(idp_sub):
                skipped_already_hashed += 1
                continue

            issuer = issuer_by_idp_name[idp_name]
            new_hash = hash_idp_sub(idp_sub, issuer)

            print(f"{uuid.UUID(bytes=row_id)}  [{idp_name}]  {idp_sub!r} -> {new_hash}")

            if args.apply:
                with conn.cursor() as cur:
                    cur.execute(
                        "UPDATE user_idp_bindings SET idp_sub = %s WHERE id = %s",
                        (new_hash, row_id),
                    )
            updated += 1

        if args.apply:
            conn.commit()
        else:
            conn.rollback()

        print(
            f"\n{'applied' if args.apply else 'dry run'}: "
            f"{updated}/{total} rows hashed, {skipped_already_hashed} already hashed"
        )
    finally:
        conn.close()


if __name__ == "__main__":
    main()
