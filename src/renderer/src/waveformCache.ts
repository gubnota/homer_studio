// Audio sources and rendered takes are immutable. Cache only completed windows;
// callers retain their own request cancellation and failed requests remain retryable.
export class WaveformCache<T> {
  private readonly entries = new Map<string, T>()
  constructor(private readonly limit = 120) {}
  get(key: string): T | undefined {
    const value = this.entries.get(key)
    if (value !== undefined) { this.entries.delete(key); this.entries.set(key, value) }
    return value
  }
  set(key: string, value: T): void {
    this.entries.delete(key); this.entries.set(key, value)
    while (this.entries.size > this.limit) this.entries.delete(this.entries.keys().next().value!)
  }
  async load(key: string, request: () => Promise<T>): Promise<T> {
    const cached = this.get(key)
    if (cached !== undefined) return cached
    const value = await request(); this.set(key, value); return value
  }
}
