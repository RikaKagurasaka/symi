import { z } from 'zod'

const githubReleaseAssetSchema = z.object({
  id: z.number(),
  name: z.string(),
  size: z.number(),
  content_type: z.string(),
  browser_download_url: z.string().url(),
  download_count: z.number(),
  updated_at: z.string(),
  label: z.string().nullable().optional()
})

const githubLatestReleaseSchema = z.object({
  id: z.number(),
  tag_name: z.string(),
  name: z.string().nullable(),
  published_at: z.string().nullable(),
  html_url: z.string().url(),
  body: z.string().nullable(),
  assets: z.array(githubReleaseAssetSchema)
})

export type GithubLatestRelease = z.infer<typeof githubLatestReleaseSchema>

function buildHeaders(token?: string, accept?: string) {
  const headers: HeadersInit = {
    'User-Agent': 'symi-docs-nitro',
    'X-GitHub-Api-Version': '2022-11-28'
  }

  if (accept) {
    headers.Accept = accept
  }

  if (token) {
    headers.Authorization = `Bearer ${token}`
  }

  return headers
}

export async function fetchGithubLatestRelease(repo: string, token?: string) {
  const response = await fetch(`https://api.github.com/repos/${repo}/releases/latest`, {
    headers: buildHeaders(token, 'application/vnd.github+json')
  })

  if (!response.ok) {
    throw createError({
      statusCode: response.status,
      statusMessage: `Failed to fetch latest release from GitHub (${response.status})`
    })
  }

  const payload = await response.json()
  return githubLatestReleaseSchema.parse(payload)
}

export async function fetchGithubReleaseAsset(repo: string, assetId: number, token?: string) {
  const response = await fetch(`https://api.github.com/repos/${repo}/releases/assets/${assetId}`, {
    headers: buildHeaders(token, 'application/octet-stream'),
    redirect: 'follow'
  })

  if (!response.ok || !response.body) {
    throw createError({
      statusCode: response.status,
      statusMessage: `Failed to download release asset from GitHub (${response.status})`
    })
  }

  return response
}
