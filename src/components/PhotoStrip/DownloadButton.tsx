interface Props {
  url: string | null;
}

export function DownloadButton({ url }: Props) {
  function handleDownload() {
    if (!url) return;
    const a = document.createElement("a");
    a.href = url;
    a.download = "purikura.png";
    a.click();
  }

  return (
    <button
      class="btn-primary disabled:opacity-40 disabled:cursor-not-allowed"
      onClick={handleDownload}
      disabled={!url}
    >
      ⬇️ ダウンロード
    </button>
  );
}
