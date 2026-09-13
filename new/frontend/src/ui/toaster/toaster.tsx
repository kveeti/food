import { Toaster as SonnerToaster } from "sonner";

import styles from "./toaster.module.css";

export function Toaster() {
  return (
    <SonnerToaster
      position="top-center"
      theme="system"
      className={styles.toaster}
      toastOptions={{
        unstyled: true,
        classNames: {
          toast: styles.toast,
          content: styles.content,
          title: styles.title,
          description: styles.description,
          icon: styles.icon,
          actionButton: styles.button,
          cancelButton: styles.button,
        },
      }}
    />
  );
}
