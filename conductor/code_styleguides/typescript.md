# TypeScript / React Code Style Guide

## Naming Conventions

- **Components**: `PascalCase` — e.g., `UsageDashboard`, `TokenTable`
- **Functions, variables**: `camelCase` — e.g., `getUsageStats`, `totalCost`
- **Constants**: `SCREAMING_SNAKE_CASE` — e.g., `MAX_RETRY_COUNT`, `API_BASE_URL`
- **Types & Interfaces**: `PascalCase` with `I` prefix for interfaces — e.g., `IUsageRecord`, `ApiResponse`
- **Files**: `kebab-case` for pages/components — e.g., `usage-dashboard.tsx`, `api-client.ts`
- **CSS classes**: `camelCase` when using CSS Modules, or follow Ant Design's BEM convention

## Code Organization

- One component per file
- Group related pages under `src/pages/<feature>/`
- Shared components under `src/components/`
- API client logic under `src/api/`
- Hooks under `src/hooks/`
- Types under `src/types/`
- Utility functions under `src/utils/`

## TypeScript

- Enable `strict` mode in `tsconfig.json`
- Prefer `interface` over `type` for object shapes
- Use `type` for unions, intersections, and utility types
- Avoid `any`; use `unknown` when type is truly indeterminate
- Use `as const` for literal types and enum-like constants
- Define explicit return types on all functions
- Use generics for reusable hooks and components

## React Patterns

- Use functional components with hooks, not class components
- Use `React.FC<Props>` for component typing
- Prefer `useState` for local UI state; `useReducer` for complex state logic
- Use `useEffect` with explicit dependency arrays; avoid empty deps unless intentional
- Use `useCallback` / `useMemo` only when profiling indicates a performance issue
- Handle loading states with a `loading` boolean and error states with a `try/catch` + error boundary
- Use Ant Design's `Table`, `Form`, `Modal`, `Spin` components rather than custom equivalents

## API Integration

- Centralize API calls in `src/api/client.ts` with a shared axios instance
- Add request/response interceptors for auth token injection and error handling
- Type all API responses with interfaces
- Use `async/await` for all API calls
- Handle 401 responses by redirecting to login

## Styling

- Use Ant Design's built-in theming (`ConfigProvider`) for global style overrides
- Prefer Ant Design layout components (`Layout`, `Row`, `Col`, `Card`) for page structure
- Avoid inline styles; use CSS Modules or Ant Design's `style` prop sparingly
- Keep custom CSS minimal; rely on Ant Design's design system

## Testing

- Use Vitest + React Testing Library for unit tests
- Test component behavior, not implementation details
- Mock API calls with `msw` (Mock Service Worker)
- One test file per component/page, co-located (e.g., `Dashboard.test.tsx`)

## Linting & Formatting

- Use ESLint with `@typescript-eslint` rules
- Use Prettier for formatting
- No `console.log` in committed code (use a logger utility or `debug` flag)
- Sort imports: external → internal → relative, each group separated by blank line
