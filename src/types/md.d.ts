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
