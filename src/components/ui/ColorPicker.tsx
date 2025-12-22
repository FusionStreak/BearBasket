// Color picker for selecting list colors
import { LIST_COLORS, type ListColor } from "../../store";

interface ColorPickerProps {
  value: ListColor;
  onChange: (color: ListColor) => void;
  label?: string;
}

export function ColorPicker({
  value,
  onChange,
  label = "Color",
}: ColorPickerProps) {
  return (
    <fieldset className="color-picker">
      <legend className="color-picker-label">{label}</legend>
      <div className="color-picker-options" role="radiogroup">
        {LIST_COLORS.map((color) => (
          <button
            key={color.value}
            type="button"
            className={`color-option ${
              value === color.value ? "color-option-selected" : ""
            }`}
            style={{ backgroundColor: color.value }}
            onClick={() => onChange(color.value)}
            aria-label={color.name}
            aria-pressed={value === color.value}
            title={color.name}
          >
            {value === color.value && (
              <span className="color-option-check" aria-hidden="true">
                ✓
              </span>
            )}
          </button>
        ))}
      </div>
    </fieldset>
  );
}
