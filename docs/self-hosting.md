# Self-Hosting MaWi Gateway

This guide covers how to deploy MaWi Gateway on your own infrastructure using Docker.

## Prerequisites

- Docker Engine (v20.10+)
- Docker Compose (v2.0+)
- Postgres Database (or use the one provided in compose)

## Quick Start (Docker Compose)

The easiest way to run MaWi is using the provided `docker-compose.yml`.

1. **Clone the repository**
   ```bash
   git clone https://github.com/mawi-ai/mawi.git
   cd mawi
   ```

2. **Start the services**
   ```bash
   docker compose up -d
   ```
   This starts:
   - `mawi-api` (Backend on port 8030)
   - `mawi-web` (Frontend on port 3001)
   - `mawi-postgres` (Database)

3. **Verify installation**
   Visit `http://localhost:3001` in your browser.

## Configuration

Environment variables can be set in `.env` file or passed to Docker.

### Key Variables
- `DATABASE_URL`: Connection string for Postgres
- `RUST_LOG`: Log level (info, debug, trace)
- `CORS_ALLOWED_ORIGINS`: Comma-separated list of allowed origins

## Production Deployment

For production, we recommend:
1. Use an external managed Postgres database (RDS, Cloud SQL).
2. Set `DATABASE_URL` to point to your managed DB.
3. Put a reverse proxy (Nginx, Traefik) in front of the api and web containers for SSL termination.
4. Scale the `mawi-api` service if needed (it is stateless).

## Troubleshooting

**Common Issues:**
- **Database connection failed**: Ensure the postgres container is healthy (`docker ps`).
- **Migrations failed**: The container automatically runs migrations on start. Check logs: `docker logs mawi-api`.
