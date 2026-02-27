import { z } from 'zod'
import { queryCollection } from '@nuxt/content/server'
import { Collections } from '@nuxt/content'
import { stringify } from 'minimark/stringify'

export default defineMcpTool({
  description: `Retrieves the full content and details of a specific documentation page.

WHEN TO USE: Use this tool when you know the EXACT path to a documentation page. Common use cases:
- User asks for a specific page: "Show me the getting started guide" → /getting-started
- User asks about a known topic with a dedicated page
- You found a relevant path from list-pages and want the full content
- User references a specific section or guide they want to read

WHEN NOT TO USE: If you don't know the exact path and need to search/explore, use list-pages first.

WORKFLOW: This tool returns the complete page content including title, description, and full markdown. Use this when you need to provide detailed answers or code examples from specific documentation pages.`,
  inputSchema: {
    path: z.string().describe('The page path from list-pages or provided by the user (e.g., /getting-started/installation)')
  },
  cache: '1h',
  handler: async ({ path: urlPath }) => {
    try {
      const event = useEvent()
      const page = await queryCollection(event, 'docs' as keyof Collections)
        .where('path', '=', urlPath)
        .select('title', 'path', 'description', 'body')
        .first()

      if (!page) {
        return {
          content: [{ type: 'text', text: 'Page not found' }],
          isError: true
        }
      }

      const result = {
        title: page.title || 'Untitled',
        path: urlPath,
        description: page.description || '',
        content:  stringify({ ...page.body, type: 'minimark' }, { format: 'markdown/html' }) || '',
        url: `/docs${urlPath}`
      }

      return {
        content: [{ type: 'text', text: JSON.stringify(result, null, 2) }]
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      console.error('get-page error:', errorMessage)
      return {
        content: [{ type: 'text', text: `Failed to get page: ${errorMessage}` }],
        isError: true
      }
    }
  }
})
