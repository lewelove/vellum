export class ScrollEngine {
  currentY: number = $state(0);
  targetSlot: number = $state(0); 
  wheelAccumulator: number = 0;
  damping: number;
  threshold: number;
  
  constructor(damping: number = 0.18, threshold: number = 40) {
    this.damping = damping;
    this.threshold = threshold;
  }

  update(rowHeight: number, dpr: number = 1) {
    const idealTargetY = this.targetSlot * rowHeight;
    const snappedTargetY = Math.round(idealTargetY * dpr) / dpr;
    
    const diff = snappedTargetY - this.currentY;
    const velocity = diff * this.damping;

    if (Math.abs(diff) < 0.01) {
      this.currentY = snappedTargetY;
    } else {
      this.currentY += velocity;
    }
  }

  handleWheel(e: WheelEvent, maxSlots: number) {
    this.wheelAccumulator += e.deltaY;
    
    if (Math.abs(this.wheelAccumulator) > this.threshold) {
      const direction = this.wheelAccumulator > 0 ? 1 : -1;
      const base = Math.round(this.targetSlot);
      
      this.targetSlot = Math.max(0, Math.min(base + direction, maxSlots));
      this.wheelAccumulator = 0;
    }
  }

  syncToSlot(slot: number) {
    this.targetSlot = slot;
  }
}
