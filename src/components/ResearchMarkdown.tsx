import { useMemo } from 'react';
import Markdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import remarkMath from 'remark-math';
import rehypeKatex from 'rehype-katex';
import 'katex/dist/katex.min.css';
import { normalizeResearchMath } from '../utils/normalizeResearchMath';

// Only standard Markdown/Math AST is rendered; no untrusted raw HTML is parsed.

export default function ResearchMarkdown({ source }: { source: string }) {
  const markdown = useMemo(() => normalizeResearchMath(source), [source]);
  if (!source.trim()) return <div className="records-preview-empty">尚无内容。可以先粘贴 ChatGPT 回答，然后查看排版效果。</div>;
  return <article className="research-markdown">
    <Markdown remarkPlugins={[remarkGfm, remarkMath]} rehypePlugins={[[rehypeKatex, { throwOnError: false, strict: 'ignore', trust: false, output: 'htmlAndMathml' }]]} skipHtml
      components={{
        a({ href, children }) {
          // Only HTTPS links are clickable; block local URI and protocol handlers.
          const safeHref = href && /^https:\/\//i.test(href) ? href : undefined;
          return safeHref ? <a href={safeHref} target="_blank" rel="noopener noreferrer">{children}</a> : <span>{children}</span>;
        },
      }}
    >{markdown}</Markdown>
  </article>;
}
