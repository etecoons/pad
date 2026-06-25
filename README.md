# Pad - Real-Time Collaborative Notepad

<p align="center">
  <img src="https://raw.githubusercontent.com/UberMetroid/pad/main/frontend/Assets/pad.png" alt="Pad Logo" width="128" height="128">
</p>

Pad is a collaborative real-time notepad and text editor designed for minimal resource usage, zero external JS library bloat, and fast load speeds. Built with a Rust (Axum/Tokio) backend and a WebAssembly (Yew) frontend.

---

## 📦 Container Registry

The Docker image is published to the following registries:

*   **Docker Hub (Recommended)**: [ubermetroid/pad](https://hub.docker.com/r/ubermetroid/pad)
*   **GitHub Container Registry (GHCR)**: [ghcr.io/ubermetroid/pad](https://github.com/UberMetroid/pad/pkgs/container/pad)

---

## 🐳 Container Installation



1. Create a `docker-compose.yml` file:

```yaml
version: '3'
services:
  pad:
    image: ubermetroid/pad:latest
    container_name: pad
    restart: unless-stopped
    ports:
      - 4402:4402
    volumes:
      - ./data:/app/data
    environment:
      - PORT=4402
      - SITE_TITLE=Pad
      - BASE_URL=http://localhost:4402
      - ALLOWED_ORIGINS=*
      - PAD_PIN=1234
      - TZ=UTC
      - ENABLE_TRANSLATION=false
      - ENABLE_THEMES=true
      - ENABLE_PRINT=true
```

2. Run the container:

```bash
docker compose up -d
```

3. Open your browser and navigate to `http://localhost:4402`.

### Building the Image Locally

To build the Docker container locally from the source files:

```bash
docker build -t ubermetroid/pad:latest .
```


---

## 📋 Configuration Options

Configure these settings inside your Docker Compose environment or container environment variables:

| Variable | Description | Default |
| :--- | :--- | :--- |
| `PORT` | The port number the backend HTTP server will bind to inside the container. | `4402` |
| `SITE_TITLE` | Custom website title rendered in navigation headers, browser tabs, and PWA manifest. *(Supports fallback `PAD_TITLE` / `PAD_SITE_TITLE`)* | `Pad` |
| `BASE_URL` | Application base URL. Essential when deploying behind reverse proxies to ensure redirect and websocket links are resolved correctly. | `http://localhost:4402` |
| `ALLOWED_ORIGINS` | Comma-separated list of allowed HTTP request origins (CORS filter). Use `*` to allow all origins. | `*` |
| `PAD_PIN` | Optional 4–10 digit PIN (numerical only) to lock access to the interface. Leave empty for public mode. *(Supports fallback `PIN`)* | None |
| `TZ` | Timezone for the container processes and logs. | `UTC` |
| `ENABLE_TRANSLATION` | Enable the multi-language / translation selector in the navigation header (true/false). | `false` |
| `ENABLE_THEMES` | Enable the Super Metroid theme selector in the navigation header (true/false). | `true` |
| `ENABLE_PRINT` | Enable the print button in the navigation header (true/false). | `true` |
| `MAX_ATTEMPTS` | Maximum PIN auth attempts allowed before rate lockout. | `5` |
| `LOCKOUT_TIME` | Bruteforce lockout duration in minutes. | `15` |
| `TRUST_PROXY` | Set true if deploying behind reverse proxy (Nginx, Cloudflare). | `false` |
| `TRUSTED_PROXY_IPS` | Comma-separated list of trusted proxy CIDRs/IPs. | None |



---

*Note: This repository was forked from [DumbPad](https://github.com/DumbWareio/DumbPad).*
