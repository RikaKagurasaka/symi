import { fetchGithubLatestRelease } from '../../../utils/github'

export default defineCachedEventHandler(async (event) => {
  setHeader(event, 'Cache-Control', 'no-cache, no-store, must-revalidate')
  setHeader(event, 'Pragma', 'no-cache')
  setHeader(event, 'Expires', '0')

  const config = useRuntimeConfig()
  const release = await fetchGithubLatestRelease(config.public.githubRepo, config.githubToken)

  return {
    repo: config.public.githubRepo,
    id: release.id,
    tag: release.tag_name,
    name: release.name || release.tag_name,
    publishedAt: release.published_at,
    htmlUrl: release.html_url,
    notes: release.body,
    assets: release.assets.map(asset => ({
      id: asset.id,
      name: asset.name,
      size: asset.size,
      contentType: asset.content_type,
      downloadCount: asset.download_count,
      updatedAt: asset.updated_at,
      browserDownloadUrl: asset.browser_download_url,
      proxyDownloadUrl: `/api/releases/assets/${asset.id}`
    }))
  }
}, {
  maxAge: 60,
  staleMaxAge: 60,
  swr: true
})
