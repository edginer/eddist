import { useCallback, useEffect, useRef, useState } from "react";

type ModalLoader<T> = () => Promise<T>;
type AnyModalLoader = () => Promise<unknown>;

interface DeferredModalOptions {
  onOpenError?: (error: unknown) => void;
}

const modalComponentPromises = new Map<AnyModalLoader, Promise<unknown>>();
const resolvedModalComponents = new Map<AnyModalLoader, unknown>();

const loadModalComponent = <T>(loader: ModalLoader<T>) => {
  const cachedPromise = modalComponentPromises.get(loader);
  if (cachedPromise) return cachedPromise as Promise<T>;

  const promise = loader()
    .then((component) => {
      resolvedModalComponents.set(loader, component);
      return component;
    })
    .catch((error) => {
      modalComponentPromises.delete(loader);
      throw error;
    });
  modalComponentPromises.set(loader, promise);
  return promise;
};

const getResolvedModalComponent = <T>(loader: ModalLoader<T>) => {
  return resolvedModalComponents.get(loader) as T | undefined;
};

export const useDeferredModal = <T>(
  loader: ModalLoader<T>,
  { onOpenError }: DeferredModalOptions = {},
) => {
  const [component, setComponent] = useState<T | null>(
    () => getResolvedModalComponent(loader) ?? null,
  );
  const [open, setOpen] = useState(false);
  const mountedRef = useRef(false);
  const hasOpenedRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const resolveComponent = useCallback(() => {
    return loadModalComponent(loader).then((loadedComponent) => {
      if (mountedRef.current) setComponent(() => loadedComponent);
      return loadedComponent;
    });
  }, [loader]);

  const prefetchModal = useCallback(() => {
    void resolveComponent().catch(() => undefined);
  }, [resolveComponent]);

  const openModal = useCallback(() => {
    hasOpenedRef.current = true;
    if (component) {
      setOpen(true);
      return;
    }

    void resolveComponent()
      .then(() => {
        if (mountedRef.current) setOpen(true);
      })
      .catch((error) => {
        if (mountedRef.current) onOpenError?.(error);
      });
  }, [component, onOpenError, resolveComponent]);

  const Modal = hasOpenedRef.current ? component : null;
  return { Modal, open, openModal, prefetchModal, setOpen };
};
