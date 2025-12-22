// Accessible Input component with proper labeling and error states
// Mobile-friendly with appropriate sizing

import { type ComponentProps, forwardRef } from "react";

interface InputProps extends ComponentProps<"input"> {
  label: string;
  error?: string;
  hint?: string;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ label, error, hint, id, className = "", ...props }, ref) => {
    const inputId = id || `input-${label.toLowerCase().replace(/\s+/g, "-")}`;
    const errorId = `${inputId}-error`;
    const hintId = `${inputId}-hint`;

    return (
      <div className="input-group">
        <label htmlFor={inputId} className="input-label">
          {label}
        </label>

        {hint && (
          <p id={hintId} className="input-hint">
            {hint}
          </p>
        )}

        <input
          ref={ref}
          id={inputId}
          className={`input ${error ? "input-error" : ""} ${className}`}
          aria-invalid={!!error}
          aria-describedby={
            [error ? errorId : null, hint ? hintId : null]
              .filter(Boolean)
              .join(" ") || undefined
          }
          {...props}
        />

        {error && (
          <p id={errorId} className="input-error-message" role="alert">
            {error}
          </p>
        )}
      </div>
    );
  }
);

Input.displayName = "Input";
