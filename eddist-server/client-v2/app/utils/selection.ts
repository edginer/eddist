/**
 * Get the currently selected text if it's within the specified element
 * @param element - The element to check if selection is within
 * @returns The selected text if within element, null otherwise
 */
export const getSelectedTextInElement = (element: HTMLElement): string | null => {
  const selection = window.getSelection();
  if (!selection || selection.rangeCount === 0) {
    return null;
  }

  const selectedText = selection.toString().trim();
  if (!selectedText) {
    return null;
  }

  const range = selection.getRangeAt(0);
  // Check if the selection is within the target element
  if (element.contains(range.commonAncestorContainer)) {
    return selectedText;
  }

  return null;
};
