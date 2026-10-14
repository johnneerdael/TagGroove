/** Keep the top menus exclusive without changing nested action-group disclosures. */
export function installMenubar(root: Document) {
  const selector = '.menubar > details.menu';
  const hovered = new WeakSet<HTMLDetailsElement>();
  const menus = () => [...root.querySelectorAll<HTMLDetailsElement>(selector)];
  const close = (except?: HTMLDetailsElement) => {
    for (const menu of menus()) {
      if (menu === except) continue;
      menu.open = false;
      hovered.delete(menu);
      menu.querySelectorAll<HTMLDetailsElement>('details[open]').forEach(child => child.open = false);
    }
  };
  const open = (menu: HTMLDetailsElement) => {
    close(menu);
    menu.open = true;
  };
  const summaryMenu = (target: EventTarget | null) => {
    const summary = target instanceof Element ? target.closest('summary') : null;
    const menu = summary?.parentElement;
    return menu?.matches(selector) ? menu as HTMLDetailsElement : undefined;
  };

  root.addEventListener('click', event => {
    const menu = summaryMenu(event.target);
    if (menu) {
      // A pointer entering a different menu has already opened it. Its first
      // click keeps it open; a subsequent click on that summary closes it.
      if (event.detail > 0 && hovered.has(menu)) event.preventDefault();
      hovered.delete(menu);
      close(menu);
    } else if (!(event.target instanceof Element && event.target.closest('.menubar'))) close();
  });
  root.addEventListener('pointerover', event => {
    if (event.pointerType !== 'mouse' || event.buttons !== 0) return;
    const menu = summaryMenu(event.target);
    if (!menu || menu.open || !menus().some(item => item.open)) return;
    open(menu);
    hovered.add(menu);
  });
  // Also cover native summary activation and programmatic opens. Toggle does
  // not bubble; ignore nested disclosures and already-closed queued events.
  root.addEventListener('toggle', event => {
    const menu = event.target;
    if (menu instanceof HTMLDetailsElement && menu.matches(selector) && menu.open) close(menu);
  }, true);
  root.addEventListener('keydown', event => {
    if (event.defaultPrevented || root.querySelector('dialog[open]')) return;
    const current = menus().find(menu => menu.open);
    if (event.key === 'Escape' && current) {
      event.preventDefault();
      close();
      current.querySelector<HTMLElement>(':scope > summary')?.focus();
      return;
    }
    const menu = summaryMenu(event.target);
    if (!menu) return;
    if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
      event.preventDefault();
      const list = menus(), index = list.indexOf(menu);
      const next = list[(index + (event.key === 'ArrowRight' ? 1 : list.length - 1)) % list.length];
      if (current) open(next);
      next.querySelector<HTMLElement>(':scope > summary')?.focus();
    } else if (event.key === 'ArrowDown') {
      event.preventDefault();
      open(menu);
      menu.querySelector<HTMLElement>(':scope > .menu-list > button:not(:disabled), :scope > .menu-list > details > summary')?.focus();
    }
  });
  root.addEventListener('focusin', event => {
    if (!(event.target instanceof Element && event.target.closest('.menubar'))) close();
  });
  root.defaultView?.addEventListener('blur', () => close());
  return { open, close };
}
