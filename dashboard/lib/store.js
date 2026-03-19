// Reactive pub/sub state store — no external dependencies.
// Subscribers are notified synchronously on set/batch.

const store = { _data: {}, _subs: {} };

/**
 * Returns the current value for a given key.
 * @param {string} key
 * @returns {*}
 */
export function get(key) {
  return store._data[key];
}

/**
 * Sets a value and notifies all subscribers for that key.
 * @param {string} key
 * @param {*} value
 */
export function set(key, value) {
  store._data[key] = value;
  const listeners = store._subs[key];
  if (listeners) {
    for (let i = 0; i < listeners.length; i++) {
      listeners[i](value, key);
    }
  }
}

/**
 * Registers a callback invoked whenever the given key changes.
 * Returns an unsubscribe function.
 * @param {string} key
 * @param {function(*, string): void} cb — receives (value, key)
 * @returns {function(): void} unsubscribe
 */
export function subscribe(key, cb) {
  if (typeof cb !== 'function') {
    throw new TypeError('subscribe callback must be a function');
  }
  if (!store._subs[key]) {
    store._subs[key] = [];
  }
  store._subs[key].push(cb);

  return () => {
    const list = store._subs[key];
    if (!list) return;
    const idx = list.indexOf(cb);
    if (idx !== -1) list.splice(idx, 1);
  };
}

/**
 * Applies multiple updates atomically — all values are written first,
 * then subscribers are notified once per affected key.
 * @param {Record<string, *>} updates
 */
export function batch(updates) {
  const changed = Object.keys(updates);

  // Write phase
  for (let i = 0; i < changed.length; i++) {
    store._data[changed[i]] = updates[changed[i]];
  }

  // Notify phase
  for (let i = 0; i < changed.length; i++) {
    const key = changed[i];
    const listeners = store._subs[key];
    if (listeners) {
      for (let j = 0; j < listeners.length; j++) {
        listeners[j](store._data[key], key);
      }
    }
  }
}

/**
 * Returns an array of all stored keys.
 * @returns {string[]}
 */
export function keys() {
  return Object.keys(store._data);
}

/**
 * Removes all data and subscribers.
 */
export function clear() {
  store._data = {};
  store._subs = {};
}
