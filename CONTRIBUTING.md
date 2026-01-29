# Contributing to MaWi Gateway

MaWi is an AI gateway built for production. We use Rust for performance and reliability, and Next.js for the frontend.

## Quick Start

The entire stack runs in Docker:

```bash
docker compose up -d
# Frontend: http://localhost:3001
# API: http://localhost:8030
```

For local development:

```bash
# Backend
cd backend
cargo build
cargo test

# Frontend
cd frontend
npm install
npm run dev
```

## Code Philosophy

**Rust Backend**:
- Prefer `Result<T>` over panics
- Use `tracing` for logging (not `println!` or `eprintln!`)
- Run `cargo fmt` and `cargo clippy` before committing
- Add tests for new features

**Frontend**:
- TypeScript strict mode always
- Functional components with hooks
- Keep components focused and under 300 lines

**General**:
- Write comments that explain WHY, not WHAT
- Small, focused PRs are easier to review
- Tests must pass before merging

## Contributing

1. **Fork the repository**
   ```bash
   git clone https://github.com/mawi-ai/mawi.git
   cd mawi
   ```

2. **Create a feature branch**
   ```bash
   git checkout -b feat/your-feature-name
   ```

3. **Make your changes**
   - Write tests for new features
   - Update documentation if needed
   - Ensure all tests pass: `cargo test && npm test`

4. **Commit with conventional commits**
   ```bash
   git commit -m "feat: add amazing feature"
   ```
   
   Use:
   - `feat:` - New feature
   - `fix:` - Bug fix
   - `docs:` - Documentation changes
   - `refactor:` - Code refactoring
   - `test:` - Adding tests
   - `chore:` - Maintenance tasks

5. **Push and create PR**
   ```bash
   git push origin feat/your-feature-name
   ```
   
   Then create a pull request with:
   - Clear description of what changed and why
   - Link to related issues
   - Screenshots (if UI changes)

## Need Help?

- Check existing issues and discussions
- Ask questions in your PR - we're friendly!
- For major changes, open an issue first to discuss

## Code of Conduct

Be respectful and professional. We welcome newcomers and focus on constructive feedback.
