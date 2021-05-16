import * as React from "react";
import { createContext, useContext, useMemo, useState } from "react";

type ToastPayload = {
  id: number;
  content: React.ReactNode;
};

export type ToastController = {
  showToast: (message: string, timeout?: number) => void;
};

const ToastContext = createContext<ToastController>(null!);

type Props = {
  children: React.ReactNode;
};

let idCount = 0;

export function ToastProvider(props: Props) {
  const [toasts, setToasts] = useState<ToastPayload[]>([]);

  const controller: ToastController = useMemo(
    () => ({
      showToast: (message: string, timeout: number = 3000) => {
        const id = idCount++;
        setToasts((ms) => [
          ...ms,
          {
            id,
            content: <div>{message}</div>,
          },
        ]);
        setTimeout(() => {
          setToasts((ms) => ms.filter((m) => m.id !== id));
        }, timeout);
      },
    }),
    [setToasts]
  );

  return (
    <ToastContext.Provider value={controller}>
      {props.children}
      <div>
        {toasts.map((t) => (
          <div key={t.id}>{t.content}</div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast() {
  return useContext(ToastContext);
}
