import { fetchGithubReleaseAsset } from '../../../../utils/github'

export default defineEventHandler(async (event) => {
  const rawAssetId = getRouterParam(event, 'assetId')
  const assetId = Number.parseInt(rawAssetId || '', 10)

  if (!Number.isFinite(assetId) || assetId <= 0) {
    throw createError({ statusCode: 400, statusMessage: 'Invalid asset id' })
  }

  const config = useRuntimeConfig()
  const upstream = await fetchGithubReleaseAsset(config.public.githubRepo, assetId, config.githubToken)

  const headers = new Headers()

  const contentType = upstream.headers.get('content-type')
  const contentLength = upstream.headers.get('content-length')
  const contentDisposition = upstream.headers.get('content-disposition')
  const etag = upstream.headers.get('etag')
  const lastModified = upstream.headers.get('last-modified')

  if (contentType) headers.set('content-type', contentType)
  if (contentLength) headers.set('content-length', contentLength)
  if (contentDisposition) headers.set('content-disposition', contentDisposition)
  if (etag) headers.set('etag', etag)
  if (lastModified) headers.set('last-modified', lastModified)

  return new Response(upstream.body, {
    status: upstream.status,
    headers
  })
})
