# Product Guidelines

## Prose Style

- **Professional & concise:** Use clear, business-appropriate language. Avoid jargon unless necessary.
- **Action-oriented:** Instructions and descriptions should focus on what the user can accomplish.
- **Bilingual (Chinese primary):** All UI text and documentation default to Chinese (Simplified). Technical terms (e.g., "model", "prompt", "token") may remain in English for clarity.
- **Data-driven:** Present information with numbers, metrics, and concrete examples whenever possible.

## Brand & Visual Identity

- **Enterprise tone:** Clean, trustworthy, and non-intrusive. The tool is background infrastructure — it should feel solid and reliable.
- **Consistent iconography:** Use Ant Design icons throughout. Each data category (security, cost, usage, quality) should have a distinct icon for quick visual scanning.
- **Color semantics:**
  - Green → healthy / within budget / positive trend
  - Yellow/Orange → warning / approaching threshold
  - Red → critical / breach / anomaly detected
  - Blue → neutral information / system default
- **Dashboard-first:** The primary interface is the dashboard. All deep-dive pages should be accessible within 2 clicks from the dashboard.

## UX Principles

- **Progressive disclosure:** Show summary metrics first (aggregated), allow drill-down into detail (individual records) only when needed.
- **Read-only by default:** Analysis module is observation-oriented. Configuration and data deletion should require explicit admin privileges.
- **Zero performance impact on proxy path:** Content collection must be fully async and never block the AI request/response flow.
- **Configurable retention:** All data retention periods must be adjustable via environment variables, not hardcoded.

## Data Presentation

- **Time series first:** Trends over time are the primary visualization type (line charts for usage, bar charts for comparison).
- **Top-N lists:** Show top 5-10 items (users, models, costs) with the rest grouped as "Other" for readability.
- **Exportable:** All chart data and tables must support CSV export for offline analysis.
- **Privacy-aware displays:** Sensitive content (prompts, responses) must be truncated in list views with an explicit "View Full" action.

## Quality Attributes

- **Searchability:** All stored content must be full-text searchable within the retention window.
- **Auditability:** Every analysis view should indicate the data time range and whether it covers full or aggregated data.
- **Performance:** Dashboards must load in under 3 seconds for 100K+ daily request volumes.
