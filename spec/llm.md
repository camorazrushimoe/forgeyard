# LLM endpoint for Pi

Onboard stores any OpenAI-compatible base URL + token:

```toml
[llm]
endpoint = "http://192.168.1.20:8080/v1"   # or api.openai.com, openrouter, a LAN box
default = ""                                 # token; local servers may use a dummy
```

No vendor enum in v0. Local Wi-Fi LLM, GPT, Claude-via-gateway — all the same shape: URL + token.

## What the wrapper exports to Pi

```
OPENAI_BASE_URL=<llm.endpoint>
OPENAI_API_KEY=<llm.default>
```

If Pi on the installed version uses different names, the wrapper aliases them to the same two values. Do not hard-code OpenRouter.

Empty endpoint → `hook stop` `fail` `summary=llm_endpoint_missing`.
Pi talks to that URL from the **laptop** (the box on the same LAN as a local model). The SSH host does not need the LLM.
