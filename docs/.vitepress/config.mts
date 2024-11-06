import fs from 'fs';
import { withMermaid } from 'vitepress-plugin-mermaid';

// https://vitepress.dev/reference/site-config
export default withMermaid({
	vue: {
		template: {
			compilerOptions: {
				// treat all tags with a dash as custom elements
				isCustomElement: tag => tag.includes('-'),
			},
		},
	},
	vite: {
		optimizeDeps: {
			include: ['mermaid'],
		},
	},
	base: '/tnesh-stack',
	title: 'TNESH stack',
	description: 'Full stack to build p2p apps with ease',
	themeConfig: {
		// https://vitepress.dev/reference/default-theme-config

		sidebar: [
			{
				text: 'Introduction',
				link: '/introduction.md',
			},
			{
				text: 'Scaffolding',
				items: [
					{
						text: 'Scaffolding a hApp',
						link: '/scaffolding-a-happ.md',
					},
					{
						text: 'Scaffolding a zome module',
						link: '/scaffolding-a-zome-module.md',
					},
				],
			},
			{
				text: 'Guides',
				items: [
					{
						text: 'Holochain',
						link: '/guides/holochain.md',
					},
					{
						text: 'Custom Elements',
						link: '/guides/custom-elements.md',
					},
					{
						text: 'Signals',
						link: '/guides/signals.md',
					},
					{
						text: 'Nix',
						link: '/guides/nix.md',
					},
					{
						text: 'Tauri',
						link: '/guides/tauri.md',
					},
				],
			},
		],

		socialLinks: [
			{
				icon: 'github',
				link: 'https://github.com/darksoil-studio/tnesh-stack',
			},
		],
		search: {
			provider: 'local',
		},
	},
	head: [
		[
			'script',
			{},
			// Synchronize the vitepress dark/light theme with the shoelace mode
			`
  function syncTheme() {
      const isDark = document.documentElement.classList.contains('dark');
      const isShoelaceDark = document.documentElement.classList.contains('sl-theme-dark');
      if (isDark && !isShoelaceDark) document.documentElement.classList = "dark sl-theme-dark";
      if (!isDark && isShoelaceDark) document.documentElement.classList = "";
  }
  const attrObserver = new MutationObserver((mutations) => {
    mutations.forEach(mu => {
      if (mu.type !== "attributes" && mu.attributeName !== "class") return;
      syncTheme();
    });
  });
  attrObserver.observe(document.documentElement, {attributes: true});
  syncTheme();
        `,
		],
	],
});
