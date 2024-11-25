import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Paydefi } from "../target/types/paydefi";

describe("paydefi", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.Paydefi as Program<Paydefi>;

  // ToDo: add tests
});
