import { canStartCapture } from "~/state/signals";

interface Props {
  onClick: () => void;
}

export function CaptureButton({ onClick }: Props) {
  const disabled = !canStartCapture.value;

  return (
    <button
      class="btn-primary px-8 py-3 text-lg disabled:opacity-40 disabled:cursor-not-allowed"
      onClick={onClick}
      disabled={disabled}
    >
      📸 撮影スタート
    </button>
  );
}
