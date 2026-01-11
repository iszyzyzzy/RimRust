import QuickLRU from 'quick-lru';

const lru = new QuickLRU<string, number>({maxSize: 100});

export function getTextWidth(text: string, style: Partial<CSSStyleDeclaration> = {}): number {
    const cacheKey = JSON.stringify({ text, style });
    if (lru.has(cacheKey)) {
        return lru.get(cacheKey)!;
    }

    const span = document.createElement('span');
    span.style.position = 'absolute';
    span.style.visibility = 'hidden';
    span.style.whiteSpace = 'nowrap';
    Object.assign(span.style, style);
    span.textContent = text;
    document.body.appendChild(span);
    const width = span.offsetWidth;
    document.body.removeChild(span);

    lru.set(cacheKey, width);
    return width;
}