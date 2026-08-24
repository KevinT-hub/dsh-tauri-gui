// TypeScript JSX declarations for Material Web custom elements
// (md-filled-button, md-list, md-circular-progress, ...).
//
// NOTE: this is NOT a Material 3 theme/color/runtime configuration — it is
// purely the JSX typing for the custom HTML elements used across the shell
// UI. Keep a single declaration file in `src/types/`; the legacy
// `md.d.ts` alias has been removed to avoid duplicate IntrinsicElements.

import type { DetailedHTMLProps, HTMLAttributes } from "react";

declare module "react" {
  namespace JSX {
    interface IntrinsicElements {
      "md-filled-button": DetailedHTMLProps<
        HTMLAttributes<HTMLElement> & { disabled?: boolean },
        HTMLElement
      >;
      "md-outlined-button": DetailedHTMLProps<
        HTMLAttributes<HTMLElement> & { disabled?: boolean },
        HTMLElement
      >;
      "md-text-button": DetailedHTMLProps<
        HTMLAttributes<HTMLElement> & { disabled?: boolean },
        HTMLElement
      >;
      "md-list": DetailedHTMLProps<HTMLAttributes<HTMLElement>, HTMLElement>;
      "md-list-item": DetailedHTMLProps<
        HTMLAttributes<HTMLElement> & {
          type?: string;
          headline?: string;
          supportingText?: string;
        },
        HTMLElement
      >;
      "md-circular-progress": DetailedHTMLProps<
        HTMLAttributes<HTMLElement> & {
          indeterminate?: boolean;
          value?: number;
          fourColor?: boolean;
        },
        HTMLElement
      >;
      "md-linear-progress": DetailedHTMLProps<
        HTMLAttributes<HTMLElement> & {
          indeterminate?: boolean;
          value?: number;
        },
        HTMLElement
      >;
    }
  }
}
