import { selectedFilter, capturedPhoto } from "~/state/signals";
import { setFilter } from "~/hooks/useAppState";
import type { FilterName } from "~/state/types";
import { FilterThumb } from "./FilterThumb";

const FILTERS: FilterName[] = ["none", "grayscale", "sepia", "vivid", "soft", "warm", "cool"];

export function FilterPanel() {
  const previewUrl = capturedPhoto.value ?? "";
  const current = selectedFilter.value;

  return (
    <div class="flex gap-2 overflow-x-auto h-full items-center pb-1">
      {FILTERS.map((f) => (
        <FilterThumb
          key={f}
          name={f}
          previewUrl={previewUrl}
          isSelected={current === f}
          onSelect={() => setFilter(f)}
        />
      ))}
    </div>
  );
}
