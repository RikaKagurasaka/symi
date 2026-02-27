import { queryCollection } from '@nuxt/content/server'

export default defineMcpTool({
  description: `Lists all available documentation pages with their categories and basic information.

WHEN TO USE: Use this tool when you need to EXPLORE or SEARCH for documentation about a topic but don't know the exact page path. Common scenarios:
- "Find documentation about markdown features" - explore available guides
- "Show me all getting started guides" - browse introductory content
- "Search for advanced configuration options" - find specific topics
- User asks general questions without specifying exact pages
- You need to understand the overall documentation structure

WHEN NOT TO USE: If you already know the specific page path (e.g., "/getting-started/installation"), use get-page directly instead.

WORKFLOW: This tool returns page titles, descriptions, and paths. After finding relevant pages, use get-page to retrieve the full content of specific pages that match the user's needs.

OUTPUT: Returns a structured list with:
- title: Human-readable page name
- path: Exact path for use with get-page
- description: Brief summary of page content
- url: Full URL for reference`,
  cache: '1h',
  handler: async () => {
    try {
      const event = useEvent()
      const pages = await queryCollection(event, 'docs')
        .select('title', 'path', 'description')
        .all()

      const result = pages.map(page => ({
        title: page.title,
        path: page.path,
        description: page.description,
        url: page.path
      }))

      return {
        content: [{ type: 'text', text: JSON.stringify(result, null, 2) }]
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      console.error('list-pages error:', errorMessage)
      return {
        content: [{ type: 'text', text: `Failed to list pages: ${errorMessage}` }],
        isError: true
      }
    }
  }
})
