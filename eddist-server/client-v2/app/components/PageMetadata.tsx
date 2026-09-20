interface PageMetadataProps {
  title: string;
  description?: string;
  noIndex?: boolean;
  siteName: string;
  canonicalUrl: string;
}

export const PageMetadata = ({
  title,
  description,
  noIndex = false,
  siteName,
  canonicalUrl,
}: PageMetadataProps) => (
  <>
    <title>{title}</title>
    {description && (
      <>
        <meta name="description" content={description} />
        <meta property="og:description" content={description} />
        <meta name="twitter:description" content={description} />
      </>
    )}
    {noIndex && <meta name="robots" content="noindex,follow" />}
    <link rel="canonical" href={canonicalUrl} />
    <meta property="og:title" content={title} />
    <meta property="og:url" content={canonicalUrl} />
    <meta property="og:site_name" content={siteName} />
    <meta property="og:type" content="website" />
    <meta name="twitter:card" content="summary" />
    <meta name="twitter:title" content={title} />
  </>
);
