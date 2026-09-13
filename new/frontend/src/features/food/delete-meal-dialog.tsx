import { AlertDialog } from "@base-ui/react/alert-dialog";
import { useRef } from "react";

import { Button } from "../../ui/button/button.tsx";
import { TrashIcon } from "../../ui/trash-icon.tsx";

import styles from "./delete-meal-dialog.module.css";

export function DeleteMealDialog(props: {
  mealName: string;
  onConfirm: () => void;
}) {
  const cancelButton = useRef<HTMLButtonElement>(null);

  return (
    <AlertDialog.Root>
      <AlertDialog.Trigger
        render={
          <Button
            type="button"
            variant="destructive"
            aria-label={`Delete ${props.mealName} meal`}
            className="px-3!"
          />
        }
      >
        <TrashIcon />
        Delete
      </AlertDialog.Trigger>
      <AlertDialog.Portal>
        <AlertDialog.Backdrop className={styles.backdrop} />
        <AlertDialog.Viewport className={styles.viewport}>
          <AlertDialog.Popup
            initialFocus={cancelButton}
            className={`${styles.popup} shadow-2xl`}
          >
            <div className="space-y-6 p-6">
              <AlertDialog.Title className="text-lg font-semibold text-gray-950">
                Delete {props.mealName}?
              </AlertDialog.Title>
              <div className="flex flex-wrap justify-end gap-2">
                <AlertDialog.Close
                  render={
                    <Button
                      ref={cancelButton}
                      variant="ghost"
                      className={styles.cancel}
                    />
                  }
                >
                  No, cancel
                </AlertDialog.Close>
                <AlertDialog.Close
                  render={<Button variant="destructive" />}
                  onClick={props.onConfirm}
                >
                  Yes, delete
                </AlertDialog.Close>
              </div>
            </div>
          </AlertDialog.Popup>
        </AlertDialog.Viewport>
      </AlertDialog.Portal>
    </AlertDialog.Root>
  );
}
