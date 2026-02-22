import { fetchGithubLatestRelease } from '../../../utils/github'

export default defineCachedEventHandler(async () => {
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
  maxAge: 60 * 5,
  staleMaxAge: 60 * 10,
  swr: true
})
