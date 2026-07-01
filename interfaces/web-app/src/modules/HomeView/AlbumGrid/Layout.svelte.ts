import { theme } from "../../../theme.svelte.ts";

export class LayoutManager {
  containerWidth: number = $state(0);

  gapX: number = $derived(theme.albumGrid["gap-x"] ?? 24);
  gapY: number = $derived(theme.albumGrid["gap-y"] ?? 12);
  cardSize: number = $derived(theme.albumGrid["cover-size"] ?? 200);
  
  creaseHeight: number = $derived(theme.albumGrid["crease-height"] ?? 0);
  
  rowHeight: number = $derived(
    this.gapY +       
    this.cardSize +     
    (theme.albumGrid["text-gap-main"] ?? 8) +  
    (theme.albumGrid["font-line-height-title"] ?? 18) +     
    (theme.albumGrid["text-gap-lesser"] ?? 2) +
    (theme.albumGrid["font-line-height-artist"] ?? 16)
  );

  cols: number = $derived(Math.max(1, Math.floor((this.containerWidth - 40 + this.gapX) / (this.cardSize + this.gapX))));
  gridWidth: number = $derived((this.cols * this.cardSize) + ((this.cols - 1) * this.gapX));

  get topOffset(): number {
    return this.creaseHeight - this.gapY;
  }

  getTotalHeight(rowCount: number): number {
    return (rowCount * this.rowHeight) + this.topOffset;
  }

  getRowY(index: number): number {
    return (index * this.rowHeight) + this.topOffset;
  }

  getVisibleIndices(scrollY: number, viewportHeight: number, rowCount: number): { start: number, end: number } {
    const buffer = 4;
    const start = Math.floor(scrollY / this.rowHeight) - buffer;
    const end = Math.ceil((scrollY + viewportHeight) / this.rowHeight) + buffer;
    
    return {
      start: Math.max(0, start),
      end: Math.min(Math.max(0, rowCount - 1), end)
    };
  }

  chunk<T>(arr: T[]): T[][] {
    const results: T[][] =[];
    const columns = this.cols;
    for (let i = 0; i < arr.length; i += columns) {
      results.push(arr.slice(i, i + columns));
    }
    return results;
  }
}

