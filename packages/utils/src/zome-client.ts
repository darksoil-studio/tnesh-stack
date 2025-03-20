import { AppCallZomeRequest, AppClient, SignalType } from '@holochain/client';
import { UnsubscribeFunction } from 'emittery';

import { isSignalFromCellWithRole } from './cell.js';

export class ZomeClient<SIGNAL_PAYLOAD> {
	constructor(
		public client: AppClient,
		public roleName: string,
		public zomeName: string,
	) {}

	onSignal(
		listener: (eventData: SIGNAL_PAYLOAD) => void | Promise<void>,
	): UnsubscribeFunction {
		return this.client.on('signal', async signal => {
			if (!(SignalType.App in signal)) return;
			const appSignal = signal[SignalType.App];
			if (
				(await isSignalFromCellWithRole(
					this.client,
					this.roleName,
					appSignal,
				)) &&
				this.zomeName === appSignal.zome_name
			) {
				listener(appSignal.payload as SIGNAL_PAYLOAD);
			}
		});
	}

	protected async callZome(fn_name: string, payload: any) {
		try {
			const req: AppCallZomeRequest = {
				role_name: this.roleName,
				zome_name: this.zomeName,
				fn_name,
				payload,
			};
			const result = await this.client.callZome(req);
			return result;
		} catch (e) {
			// Just retry if this is about concurrent init calls
			if (
				JSON.stringify(e).includes(
					'Another zome function has triggered the `init()` callback',
				)
			) {
				return this.callZome(fn_name, payload);
			} else {
				throw e;
			}
		}
	}
}
