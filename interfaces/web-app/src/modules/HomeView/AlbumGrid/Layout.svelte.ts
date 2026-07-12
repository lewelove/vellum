import { config } from "../../../config.svelte.ts";

export class LayoutManager {
  containerWidth: number = $state(0);

  get gapX() { return config.album_grid.spacing.x; }
  get gapY() { return config.album_grid.spacing.y; }
  get cardSize() { return config.album_grid.album_card.cover.size; }
  
  get creaseHeight() { return config.album_grid.spacing.top; }
  
  get rowHeight() {
    let textHeight = 0;
    if (config.album_grid.album_card.text.enable) {
      textHeight = config.album_grid.album_card.text.spacing.top + 
                   Math.round(config.album_grid.album_card.text.title.size * 1.2) + 
                   config.album_grid.album_card.text.spacing.middle +
                   Math.round(config.album_grid.album_card.text.albumartist.size * 1.2);
    }
    return this.gapY + this.cardSize + textHeight;
  }

  get cols() { return Math.max(1, Math.floor((this.containerWidth - 40 + this.gapX) / (this.cardSize + this.gapX))); }
  get gridWidth() { return (this.cols * this.cardSize) + ((this.cols - 1) * this.gapX); }

  get topOffset() { return this.creaseHeight - this.gapY; }

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
    const results: T[][] = [];
    const columns = this.cols;
    for (let i = 0; i < arr.length; i += columns) {
      results.push(arr.slice(i, i + columns));
    }
    return results;
  }
}
