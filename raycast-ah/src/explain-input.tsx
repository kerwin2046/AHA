import { Detail, showToast, Toast, ActionPanel, Action, LaunchProps } from "@raycast/api";
import { useEffect, useState } from "react";
import { explain, ExplainResult } from "./ah";

interface Arguments {
  query?: string;
}

export default function Command(props: LaunchProps<{ arguments: Arguments }>) {
  const initialQuery = props.arguments.query?.trim() || "";

  const [state, setState] = useState<{
    loading: boolean;
    result?: ExplainResult & { raw: string };
    error?: string;
  }>(() => ({
    loading: !!initialQuery,
    error: initialQuery ? undefined : "Type a word or code to explain, then press ⏎",
  }));

  useEffect(() => {
    if (!initialQuery) return;

    (async () => {
      try {
        setState({ loading: true, error: undefined });
        const result = explain(initialQuery);
        setState({ loading: false, result });
      } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : String(e);
        setState({ loading: false, error: msg });
        await showToast({ title: "ah failed", message: msg, style: Toast.Style.Failure });
      }
    })();
  }, [initialQuery]);

  if (state.loading) {
    return <Detail markdown="Loading..." />;
  }

  if (state.error) {
    return (
      <Detail
        markdown={`# ah\n\n${state.error}`}
        actions={
          <ActionPanel>
            <Action.OpenInBrowser url="https://github.com/USER/ah" title="View ah Docs" />
          </ActionPanel>
        }
      />
    );
  }

  const r = state.result!;
  let md = `# ${r.translation || "Explanation"}`;
  if (r.explanation) md += `\n\n> ${r.explanation}`;
  if (r.usage) md += `\n\n---\n\n### Usage\n\n\`\`\`\n${r.usage}\n\`\`\``;

  return (
    <Detail
      markdown={md}
      actions={
        <ActionPanel>
          <Action.CopyToClipboard content={r.raw} title="Copy Result" />
          <Action.CopyToClipboard content={r.translation} title="Copy Translation" />
        </ActionPanel>
      }
    />
  );
}
