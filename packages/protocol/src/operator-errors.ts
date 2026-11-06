/**
 * An error Bridge showed the person, sent to the Fleet's Operator. `crates/ipc/operations/report_bridge_error.toml`.
 */

/** One structured field of the error card, already flattened to a string. */
export type BridgeErrorField = {
  key: string;
  value: string;
};

/**
 * `report_bridge_error` (`POST /operator/errors`): what the failure card carries. `fields`, `chain` and `logs` may be
 * absent. `chain` is outermost cause first, `logs` are paths, `at` is RFC 3339 on Bridge's clock.
 */
export type ReportBridgeError = {
  code: string;
  message: string;
  fields?: BridgeErrorField[];
  chain?: string[];
  logs?: string[];
  commit?: string;
  job_id?: string;
  at: string;
};

/**
 * What `report_bridge_error` answers, always 200: `id` is the arrival's (`error:<fingerprint>:<local day>`) and
 * `repeated` says the same error was already with the Operator that day. While the Operator is off it is accepted and
 * let go, `repeated` false.
 */
export type BridgeErrorTaken = {
  id: string;
  repeated: boolean;
};
