/**
 * Verrou d'exclusion mutuelle minimal.
 *
 * `bun:sqlite` expose une connexion unique : les transactions doivent être
 * sérialisées pour qu'un `BEGIN` n'englobe pas les requêtes d'une autre
 * transaction concurrente.
 */
export class Mutex {
  #queue: (() => void)[] = [];
  #locked = false;

  async acquire(): Promise<() => void> {
    if (!this.#locked) {
      this.#locked = true;
      return this.#release;
    }
    await new Promise<void>((resolve) => this.#queue.push(resolve));
    return this.#release;
  }

  #release = (): void => {
    const next = this.#queue.shift();
    if (next) {
      next();
      return;
    }
    this.#locked = false;
  };

  get locked(): boolean {
    return this.#locked;
  }
}
