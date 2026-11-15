import * as React from "react"
import { cn } from "cn"
import { fullYear } from "@/lib/format"

/** Puts `value` into a controlled input the way typing does, so React's onChange sees it. */
function setValue(input: HTMLInputElement, value: string) {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(input, value)
  input.dispatchEvent(new Event("input", { bubbles: true }))
}

function Input({ className, type, onBlur, ...props }: React.ComponentProps<"input">) {
  return (
    <input
      type={type}
      onBlur={(event) => {
        // A two-digit year becomes 20YY once the field is left, not while it is typed.
        const fixed = type === "date" ? fullYear(event.currentTarget.value) : null
        if (fixed) setValue(event.currentTarget, fixed)
        onBlur?.(event)
      }}
      data-slot="input"
      className={cn(
        "h-8 w-full min-w-0 rounded-lg border border-input bg-transparent px-2.5 py-1 text-base transition-colors outline-none file:inline-flex file:h-6 file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:cursor-not-allowed disabled:bg-input/50 disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-3 aria-invalid:ring-destructive/20 md:text-sm dark:bg-input/30 dark:disabled:bg-input/80 dark:aria-invalid:border-destructive/50 dark:aria-invalid:ring-destructive/40",
        className
      )}
      {...props}
    />
  )
}

export { Input }
