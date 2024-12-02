import { AppClient, AppWebsocket } from '@holochain/client';
import { provide } from '@lit/context';
import { msg } from '@lit/localize';
import '@shoelace-style/shoelace/dist/components/spinner/spinner.js';
import { LitElement, css, html } from 'lit';
import { customElement, property, state } from 'lit/decorators.js';

import { appClientContext } from '../context.js';
import { sharedStyles } from '../shared-styles.js';
import './display-error.js';

@customElement('app-websocket-context')
export class AppWebsocketContext extends LitElement {
	@provide({ context: appClientContext })
	@property({ type: Object })
	client!: AppClient;

	@state() _loading = true;
	@state() _error: unknown | undefined;

	async firstUpdated() {
		try {
			this.client = await AppWebsocket.connect();
		} catch (e: unknown) {
			this._error = e;
		} finally {
			this._loading = false;
		}
	}

	render() {
		if (this._loading)
			return html`<div
				class="row"
				style="flex: 1; height: 100%; align-items: center; justify-content: center;"
			>
				<sl-spinner style="font-size: 2rem"></sl-spinner>
			</div>`;

		if (this._error)
			return html`
				<div
					style="flex: 1; height: 100%; align-items: center; justify-content: center;"
				>
					<display-error
						.error=${this._error}
						.headline=${msg('Error connecting to holochain.')}
					>
					</display-error>
				</div>
			`;

		return html`<slot></slot>`;
	}

	static styles = [
		sharedStyles,
		css`
			:host {
				display: contents;
			}
		`,
	];
}
