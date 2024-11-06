# Signals

Signals are the reactivity primitive for state management in the frontend of TNESH apps and modules.

Learn more about them [here](https://github.com/proposal-signals/signal-polyfill).


## Stores

A store is a typescript class that maintains the shared state for the elements exported by a module. This shared state is crucial in order to have important optimizations, like skipping zome calls because we already have the necessary information, or updating all the elements at once reacting to a signal that came from the zome.

After exploring different state management alternatives, like `graphql`, `redux`, `mobx` or svelte stores, we settled on signals as the base engine to build the stores.

Signals are a really simple and framework agnostic state management layer. They expose a simple `.get()` method that can be plugged into an element to get automatic updates and rerenders when the underlying data changes. You can read more about them [here](https://svelte.dev/tutorial/writable-stores).

This is a sample of using the `ProfilesStore` without any UI framework:

```ts
import { AppClient } from "@holochain/client";
import { ProfilesStore } from "@darksoil-studio/profiles-zome";
import { toPromise } from "@tnesh-stack/signals";

const appClient: AppClient = createClient();

const store = new ProfilesStore(appClient, config);

const allProfiles = await toPromise(store.allProfiles);
console.log(`These are all the profiles that exist in this app: `, allProfiles);
```

The store constructor usually accepts an [AppClient object](https://www.npmjs.com/package/@holochain/client) that allows for usage of the module in both native Holochain and Holo contexts. It can also accept module wide configuration, that will be read by any of the components of the module and may affect their behaviour.
