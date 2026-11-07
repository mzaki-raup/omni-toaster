# Security rules

Read [docs/SECURITY.md](../../docs/SECURITY.md) before changing anything
here.

## Portability

All app data goes under `OmniToasterData/` beside the exe. Never the registry,
never `%APPDATA%`, never `%TEMP%`. Never commit `OmniToasterData/`.

## Secrets

- Never plaintext. A secret lives in the encrypted vault and is referenced as
  `secret:<ulid>`.
- Redact secrets in logs and in history.
- A vault-locked save must return an error, not silently drop the value.
- Any test that writes a secret must grep the data folder afterwards to prove
  nothing leaked.

