# Custom Elements

All the elements exported by TNESH modules are [Custom Elements](https://developers.google.com/web/fundamentals/web-components/customelements), which means that they can be used in any UI framework because the browser itself can understand and render them. This means that you can use your favourite framework or web-based tooling, and still import and use the elements from the modules.

The modules are really careful to be application agnostic: they don't define any routes, or global CSS, or use any globally defined objects. This means that you'll be able to import them without interfering with the rest of your application.

Most elements are built using [lit](https://lit.dev). But any other technology that exports Custom Elements will work just fine.

To include elements into your app you import them like this:

```js
import "@darksoil-studio/profiles-zome/dist/elements/profile-prompt.js";
import "@darksoil-studio/profiles-zome/dist/elements/list-profiles.js";
```

## Context

As our elements are not just presentational, but hey also make calls to the backend by themselves via the store, we need a way to define and configure the store outside the elements, and then pass it down to the elements so that they can use it. Inspired by other frontend frameworks (React or Svelte), we use the context pattern as a way to inject the stores to the elements themselves.

In summary, the context pattern consists of defining a `<*-context>` (e.g. `<profiles-context>`) element that contains the store object, and that must be wrapping all the elements that need the store. When each element starts, it fires an event which gets captured by the closest `<*-context>` element, and it injects the store to the requesting element.

Each module exports its own `<*-context>` element, which knows how to provide the store to the elements from that same module. You just need to set its `store` property to the store object you have initialized,

To define the `<*-context>` element you can just import it:

```js
// This can be placed in the index.js, at the top level of your web-app.
import "@darksoil-studio/profiles-zome/dist/elements/profiles-context.js";
```

And then in your html:

```html
<profiles-context .store=${profilesStore}>
  <list-profiles></list-profiles>
  <search-agent></search-agent>
</profiles-context>
```

> [!NOTE]
> Here, every framework has a different style of passing a property down to the component, but they all will work fine. See [Integration with Frameworks](/guides/custom-elements.md) for examples of integrations in each of the frontend frameworks.

As you may have guessed, context providers can be nested inside each other, to provide multiple contexts to elements that could need them:

```html
<profiles-context .store=${profilesStore}>
  <invitations-context .store=${someOtherStore}>
    <list-profiles></list-profiles>
    <some-other-element></some-other-element>
  </invitations-context>
</profiles-context>
```

Go [here](https://www.npmjs.com/package/@lit-labs/context) to read more about the context library we use. 

**If you only need one component**, you don't have to use the context pattern at all. You can just pass the store as a property to that component directly:

```html
<list-profiles .store=${profilesStore}></list-profiles>
```
