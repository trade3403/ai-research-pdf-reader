/**
 * ChatGPT often uses \(..\) and \[..\] math delimiters, while remark-math
 * parses dollar-delimited math. Translate for presentation only; DB source is unchanged.
 * Do not transform fenced code or inline code spans.
 */
export function normalizeResearchMath(source: string): string {
  const lines = source.replace(/\r\n?/g, '\n').split('\n');
  let fence: { marker: string; length: number } | null = null;
  const output: string[] = [];

  for (const line of lines) {
    const match = /^ {0,3}(`{3,}|~{3,})/.exec(line);
    if (fence) {
      output.push(line);
      if (match && match[1][0] === fence.marker && match[1].length >= fence.length &&
          line.slice(match[0].length).trim() === '') fence = null;
      continue;
    }
    if (match) {
      fence = { marker: match[1][0], length: match[1].length };
      output.push(line);
      continue;
    }

    let normalized = '';
    let inlineTicks = 0;
    for (let i = 0; i < line.length;) {
      if (line[i] === '`') {
        let end = i;
        while (line[end] === '`') end++;
        const run = end - i;
        if (inlineTicks === 0) inlineTicks = run;
        else if (inlineTicks === run) inlineTicks = 0;
        normalized += line.slice(i, end);
        i = end;
        continue;
      }
      const isEscaped = i > 0 && line[i - 1] === '\\';
      if (inlineTicks === 0 && !isEscaped && line[i] === '\\' && i + 1 < line.length) {
        const next = line[i + 1];
        if (next === '(' || next === ')') {
          normalized += '$';
          i += 2;
          continue;
        }
        if (next === '[' || next === ']') {
          normalized += '$$';
          i += 2;
          continue;
        }
      }
      normalized += line[i];
      i++;
    }
    output.push(normalized);
  }
  return output.join('\n');
}
