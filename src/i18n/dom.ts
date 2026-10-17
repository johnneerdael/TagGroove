// Translates the rendered UI in place. The app renders English markup; text
// nodes and the attributes people read (title, aria-label, placeholder, …) are
// looked up in the active catalog as they appear, so every view, menu and
// dialog follows the language without building its strings differently.
// Regions marked translate="no" (file/tag values, user labels, paths) are left
// alone, as are form values. English installs nothing. An element marked
// data-verbatim="<name>" shows a name someone entered (in its text or its
// attributes, alone or inside a sentence such as "Show {name}"): that name is
// never translated, even when it equals a key, while the rest of the element's
// text is. A bare data-verbatim protects every value of the element.
import type {Translator} from './core';

const ATTRIBUTES = ['title', 'aria-label', 'aria-description', 'aria-roledescription', 'aria-valuetext', 'placeholder', 'alt', 'label', 'data-hint', 'data-tooltip'];
const SKIP = 'script,style,svg,[translate="no"],[contenteditable="true"]';
const BUTTON_INPUT = /^(button|submit|reset)$/i;

type Shown = {source: string; shown: string};

export class DomTranslator {
  private texts = new WeakMap<Text, Shown>();
  private attributes = new WeakMap<Element, Map<string, Shown>>();
  private observer?: MutationObserver;
  private pending: MutationRecord[] = [];

  constructor(private root: HTMLElement, private translator: () => Translator) {}

  /** Start following the UI (when the language needs it) and translate what is shown now. */
  apply() {
    const active = this.translator().active;
    if (active && !this.observer) {
      this.observer = new MutationObserver(records => this.mutated(records));
      this.observer.observe(this.root, {subtree: true, childList: true, characterData: true, attributes: true, attributeFilter: [...ATTRIBUTES, 'value']});
    }
    this.flush();
    this.walk(this.root);
    this.flush();
    if (!active) {this.observer?.disconnect();this.observer = undefined;}
  }

  /** Translate a subtree now, for markup measured before the observer runs. */
  translateTree(node: Node) {if (this.translator().active) {this.flush();this.walk(node);this.flush();}}

  private skipped(element: Element | null) {return !!element?.closest(SKIP);}

  private mutated(records: MutationRecord[]) {
    // Every observed application write is a new source, even when it equals
    // our last output. Invalidate the whole batch before producing any output.
    for (const record of records) {
      if (record.type === 'characterData') this.texts.delete(record.target as Text);
      else if (record.type === 'attributes' && record.attributeName) this.attributes.get(record.target as Element)?.delete(record.attributeName);
    }
    for (const record of records) {
      if (record.type === 'childList') record.addedNodes.forEach(node => {if (node.isConnected) this.walk(node);});
      else if (record.type === 'characterData') {const node = record.target as Text;if (!this.skipped(node.parentElement)) this.text(node);}
      else if (record.type === 'attributes' && record.attributeName) {
        const element = record.target as Element;
        if (!this.skipped(element)) this.attribute(element, record.attributeName);
      }
    }
    this.flush();
  }

  private flush() {
    const records = this.pending.splice(0);
    records.push(...this.observer?.takeRecords() ?? []);
    if (records.length) this.mutated(records);
  }

  /** Suppress exactly our own write, retaining any other queued application mutations. */
  private write(target: Node, attribute: string | undefined, change: () => void) {
    this.pending.push(...this.observer?.takeRecords() ?? []);
    change();
    let own = true;
    for (const record of this.observer?.takeRecords() ?? []) {
      if (own && record.target === target && (attribute === undefined ? record.type === 'characterData' : record.type === 'attributes' && record.attributeName === attribute)) own = false;
      else this.pending.push(record);
    }
  }

  private walk(start: Node) {
    if (start.nodeType === Node.TEXT_NODE) {if (!this.skipped(start.parentElement)) this.text(start as Text);return;}
    if (!(start instanceof Element) || this.skipped(start)) return;
    const walker = document.createTreeWalker(start, NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT, {
      acceptNode: node => node instanceof Element && node.matches(SKIP) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT,
    });
    for (let node: Node | null = walker.currentNode; node; node = walker.nextNode()) {
      if (node.nodeType === Node.TEXT_NODE) this.text(node as Text);
      else for (const name of ATTRIBUTES) if ((node as Element).hasAttribute(name)) this.attribute(node as Element, name);
      if (node instanceof HTMLInputElement && BUTTON_INPUT.test(node.type)) this.attribute(node, 'value');
    }
  }

  private next(current: string, record: Shown | undefined, element: Element | null) {
    // Text the app wrote since the last pass is English; text we wrote is replaced from its source.
    const source = record && record.shown === current ? record.source : current;
    const marked = element?.closest('[data-verbatim]'), name = marked?.getAttribute('data-verbatim');
    return {source, shown: this.translator().translate(source, 0, marked ? name || true : false)};
  }

  private text(node: Text) {
    // Textarea children define editable defaults; only their UI attributes translate.
    if (node.parentElement?.closest('textarea')) return;
    const current = node.data;
    const record = this.texts.get(node);
    if (!record && !/\p{L}/u.test(current)) return;
    const next = this.next(current, record, node.parentElement);
    this.texts.set(node, next);
    if (next.shown !== current) this.write(node, undefined, () => {node.data = next.shown;});
  }

  private attribute(element: Element, name: string) {
    if (name === 'value' && !(element instanceof HTMLInputElement && BUTTON_INPUT.test(element.type))) return;
    const current = element.getAttribute(name);
    let map = this.attributes.get(element);
    const record = map?.get(name);
    if (current === null || !record && !/\p{L}/u.test(current)) return;
    if (!map) {map = new Map();this.attributes.set(element, map);}
    const next = this.next(current, record, element);
    map.set(name, next);
    if (next.shown !== current) this.write(element, name, () => element.setAttribute(name, next.shown));
  }
}
