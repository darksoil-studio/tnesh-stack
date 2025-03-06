import { localized, msg } from '@lit/localize';
import '@shoelace-style/shoelace/dist/components/dropdown/dropdown.js';
import '@shoelace-style/shoelace/dist/components/input/input.js';
import SlInput from '@shoelace-style/shoelace/dist/components/input/input.js';
import '@shoelace-style/shoelace/dist/components/menu-item/menu-item.js';
import '@shoelace-style/shoelace/dist/components/menu/menu.js';
import '@shoelace-style/shoelace/dist/components/skeleton/skeleton.js';
import {
	AsyncComputed,
	AsyncState,
	Signal,
	SignalWatcher,
	fromPromise,
} from '@tnesh-stack/signals';
import { LitElement, TemplateResult, html } from 'lit';
import { customElement, property, state } from 'lit/decorators.js';

import { FormField, FormFieldController } from '../form-field-controller.js';
import { sharedStyles } from '../shared-styles.js';
import './display-error.js';

@localized()
@customElement('sl-combobox')
export class SlCombobox extends SignalWatcher(LitElement) implements FormField {
	@property()
	label: string | undefined;

	@property()
	name: string | undefined;

	@property()
	defaultValue: string | undefined;

	@property({ type: Boolean, attribute: 'required' })
	required = false;

	@property({ type: Boolean, attribute: 'disabled' })
	disabled = false;

	_controller = new FormFieldController(this);

	@property()
	search!: (filter: string) => Promise<string[]>;

	@property()
	renderItem: ((searchResult: string) => TemplateResult) | undefined;

	_search = new AsyncState<string[]>({ status: 'pending' });

	async performSearch(filter: string) {
		this._search.set({ status: 'pending' });
		try {
			const results = await this.search(filter);
			this._search.set({ status: 'completed', value: results });
		} catch (e) {
			this._search.set({ status: 'error', error: e });
		}
	}

	reportValidity() {
		if (this.disabled) return true;
		return true;
	}

	value: string | undefined;

	reset() {
		this.value = this.defaultValue;
		setTimeout(() => {
			if (this.defaultValue) {
				this.shadowRoot!.querySelector('sl-input')!.value = this.defaultValue;
			}
		});
	}

	renderSearchResults() {
		const input = this.shadowRoot!.querySelector('sl-input');

		if (!input?.value)
			return html`<sl-menu-item disabled
				>${msg('Enter a filter to search.')}</sl-menu-item
			>`;
		const searchResult = this._search.get();
		console.log(searchResult);
		switch (searchResult.status) {
			case 'pending':
				return Array.from(Array(3)).map(
					() => html`
						<sl-menu-item>
							<sl-skeleton
								effect="sheen"
								style="width: 100px; margin: 8px; border-radius: 12px"
							></sl-skeleton>
						</sl-menu-item>
					`,
				);
			case 'error':
				return html`
					<display-error
						style="flex: 1; display:flex"
						tooltip
						.headline=${msg('Error searching.')}
						.error=${searchResult.error}
					></display-error>
				`;
			case 'completed': {
				const results = searchResult.value;

				if (results.length === 0)
					return html`<sl-menu-item disabled>
						${msg('Search returned no matches.')}
					</sl-menu-item>`;

				return html`
					${results.map(
						result => html`
							<sl-menu-item .value=${result}
								>${this.renderItem ? this.renderItem(result) : result}
							</sl-menu-item>
						`,
					)}
				`;
			}
		}
	}

	render() {
		return html`
			<sl-dropdown hoist>
				<sl-input
					slot="trigger"
					.label=${this.label}
					.required=${this.required}
					.disabled=${this.disabled}
					@input=${(e: CustomEvent) => {
						this.requestUpdate();
						this.performSearch((e.target as SlInput).value);
						this.shadowRoot!.querySelector('sl-dropdown')!.show();
					}}
				>
				</sl-input>

				<sl-menu
					@sl-select=${async (e: CustomEvent) => {
						this.value = e.detail.item.value;
						this.shadowRoot!.querySelector('sl-input')!.value = this.value;
						this.dispatchEvent(
							new CustomEvent('sl-change', {
								bubbles: true,
								composed: true,
							}),
						);
					}}
					>${this.renderSearchResults()}
				</sl-menu>
			</sl-dropdown>
		`;
	}

	static styles = sharedStyles;
}
