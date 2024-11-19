# TNESH stack

The TNESH stack is a full opinionated stack to build fully distributed p2p apps quickly and easily.

It is composed of these components: 

- **T**auri
- **N**ix
- Custom **E**lements
- **S**ignals
- **H**olochain

At darksoil, we have been researching into the best patterns to create reusable modules to create holochain hApps. The TNESH stack is the result of years of research and refinement, and we think it's now ready to be presented in a more public manner and used by other developers.

## Modularity

p2p apps need robust cryptographic security mechanisms, since they cannot rely on a central server dictating the state of the app.

As such, the need for modular and battle tested building blocks is much greater in p2p context, since it can be costly to develop these in a secure way. TNESH is carefully designed for creating secure and efficient reusable modules that can be integrated together to create a hApp really quickly.


### Existing Modules

- [profiles-zome](https://darksoil.studio/profiles-zome)
- [linked-devices-zome](https://darksoil.studio/linked-devices-zome)
- [file-storage-zome](https://darksoil.studio/file-storage)
- [notifications-zome](https://darksoil.studio/notifications-zome)
- [roles-zome](https://darksoil.studio/roles-zome)

Other TNESH modules like mutual-credit, tasks or booking of time slots are being developed.


## Distribution

p2p apps can also have a hard time with distribution of their code to users in a user friendly way. Challenges like code updates or user experience during the setup of the app can be really hard to overcome by small teams of developers, who would much rather be focused in developing the features that the end user needs.

TNESH also offers tooling to distribute p2p apps in a reliable and seamless way. We use [nix](https://nixos.org/) as our package manager to guarantee reliable code building, and [tauri](https://tauri.apps) to package and distribute your app for desktop and mobile platforms.
