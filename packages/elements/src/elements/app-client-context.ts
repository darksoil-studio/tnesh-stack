import { AppClient, AppWebsocket } from '@holochain/client';
import { provide } from '@lit/context';
import '@shoelace-style/shoelace/dist/components/spinner/spinner.js';
import { LitElement, css, html } from 'lit';
import { customElement, property, state } from 'lit/decorators.js';

import { appClientContext } from '../context.js';

@customElement('app-client-context')
export class AppClientContext extends LitElement {
	@provide({ context: appClientContext })
	@property({ type: Object })
	client!: AppClient;

	render() {
		return html`<slot></slot>`;
	}

	static styles = css`
		:host {
			display: contents;
		}
	`;
}
