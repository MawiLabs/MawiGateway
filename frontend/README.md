# MaWi Frontend - React Flow Visualization

Next.js app with React Flow architecture visualization and backend integration.

## Setup

```bash
cd frontend
npm install
npm run dev
```

Visit: http://localhost:3001

## Architecture

### Components

- **ExecutionFlowCanvas**: Main React Flow canvas with backend data fetching
- **UserNode**: User request node
- **GatewayNode**: Central MaWi gateway with model list
- **ServiceNode**: Service nodes (chat/audio/video)
- **ModelNode**: Leader and worker model nodes

### Backend Integration

API proxy configured in `next.config.js`:
- Frontend calls `/api/v1/services` → Backend `http://localhost:8030/v1/services`
- Real-time service and model data from database

### Styling

- **Dark Mode**: `bg-gray-900` base
- **Gateway**: Blue theme (`bg-blue-900`, `border-blue-400`)
- **Services**: Color-coded by type (green=chat, purple=audio, red=video)
- **Leader Models**: Teal (`bg-teal-900`, `border-teal-400`)
- **Failover Edges**: Dashed orange lines

## Features

✅ Real-time backend data
✅ Service type visualization
✅ Leader-worker pattern
✅ Failover chain
✅ Guardrails indicator
✅ Static, diagram-like layout
✅ LIVE status indicator
