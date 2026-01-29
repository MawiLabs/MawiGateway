# MaWi Node.js SDK

Official Node.js/TypeScript client for MaWi Gateway.

## Installation

```bash
npm install
```

## Usage

See `example.ts` for a quick start.

To run the example:
```bash
npm run example
```

### Basic Usage

```typescript
import { MaWiClient } from './MaWiClient';

const client = new MaWiClient({
    BASE: 'http://localhost:8030/v1',
    HEADERS: { 'Authorization': 'Bearer YOUR_TOKEN' }
});

const response = await client.chat.chatCompletions({
    service: 'openai-gpt-4',
    messages: [{ role: 'user', content: 'Hello' }]
});
```
