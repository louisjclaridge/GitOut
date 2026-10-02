// A single global right-click menu.

export type MenuItem =
  | { label: string; icon?: string; action: () => void; danger?: boolean; disabled?: boolean }
  | "sep";

export const menu = $state({
  open: false,
  x: 0,
  y: 0,
  items: [] as MenuItem[],
});

export function showMenu(e: MouseEvent, items: MenuItem[]) {
  e.preventDefault();
  e.stopPropagation();
  menu.items = items;
  menu.x = e.clientX;
  menu.y = e.clientY;
  menu.open = true;
}

export function closeMenu() {
  menu.open = false;
}
