import { useState } from "react";

export function useCrudModalState<T>() {
  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const [editingItem, setEditingItem] = useState<T | undefined>();

  return {
    isCreateOpen,
    openCreate: () => setIsCreateOpen(true),
    closeCreate: () => setIsCreateOpen(false),
    isEditOpen: editingItem !== undefined,
    editingItem,
    openEdit: (item: T) => setEditingItem(item),
    closeEdit: () => setEditingItem(undefined),
  };
}
