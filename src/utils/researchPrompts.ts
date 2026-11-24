export type ResearchPromptKind = 'explain' | 'math' | 'novelty' | 'python';

/** User-confirmed copy/paste only. No ChatGPT page scripts or automated submission. */
export function buildResearchPrompt(kind: ResearchPromptKind, quote: string, filename: string, page: number): string {
  const excerpt = quote.trim().slice(0, 20_000);
  if (!excerpt) throw new Error('必须先选择 PDF 内容');
  const tasks: Record<ResearchPromptKind, string> = {
    explain: '请用中文解释以下科研论文选段，区分作者原文结论与自己的推断，并标明关键定义和假设。',
    math: '请检查选段中的数学模型、公式含义、符号定义、边界条件与推导逻辑，指出未提供的条件；不得虚构公式。',
    novelty: '请分析选段涉及的研究问题、已有方法的可能局限与可验证的潜在创新方向。仅凭选段不能证明原创性，明确哪些观点仍需检索文献核实。',
    python: '请根据选段提出可复现实验方案，包括输入参数、基线算法、误差指标和稳健性检查，并给出可运行的 Python 示例；不确定参数必须明确标为假设。',
  };
  return `${tasks[kind]}\n\n论文来源：${filename.replace(/[\r\n]/g,' ')}\nPDF 页码：${page}\n\n【论文选段】\n${excerpt}\n【选段结束】\n\n请不要把论文中的断言自动当作事实。`;
}
