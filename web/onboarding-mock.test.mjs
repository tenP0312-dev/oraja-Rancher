import assert from "node:assert/strict";
import test from "node:test";

import {
  completedOnboardingMockSteps,
  createOnboardingMockState,
  onboardingMockStepState,
  reduceOnboardingMock,
  validateOnboardingMockAccount
} from "./onboarding-mock.mjs";

test("the mock advances only through the ordered onboarding flow", () => {
  let state = createOnboardingMockState();
  assert.equal(state.stage, "account");
  assert.equal(completedOnboardingMockSteps(state), 0);

  state = reduceOnboardingMock(state, {type: "CONFIGURE_PROFILE", profileId: "BMSIR"});
  assert.equal(state.stage, "account");

  state = reduceOnboardingMock(state, {
    type: "COMPLETE_ACCOUNT",
    accountId: "190000",
    accountName: "NEW_PLAYER"
  });
  assert.equal(state.stage, "profile");
  assert.equal(state.accountName, "NEW_PLAYER");
  assert.equal(completedOnboardingMockSteps(state), 1);
  assert.equal(onboardingMockStepState(state, "account"), "complete");

  state = reduceOnboardingMock(state, {type: "CONFIGURE_PROFILE", profileId: "BMSIR"});
  assert.equal(state.stage, "connection");
  assert.equal(state.profileId, "BMSIR");
  assert.equal(completedOnboardingMockSteps(state), 2);

  state = reduceOnboardingMock(state, {type: "START_CONNECTION_TEST"});
  assert.equal(state.connectionTesting, true);
  state = reduceOnboardingMock(state, {type: "COMPLETE_CONNECTION_TEST"});
  assert.equal(state.stage, "ready");
  assert.equal(state.connectionPassed, true);
  assert.equal(completedOnboardingMockSteps(state), 4);
});

test("the existing-account path and reset keep the mock session-only", () => {
  let state = createOnboardingMockState();
  state = reduceOnboardingMock(state, {type: "SELECT_ACCOUNT_MODE", mode: "existing"});
  state = reduceOnboardingMock(state, {
    type: "COMPLETE_ACCOUNT",
    accountId: "190123",
    accountName: "BMSIR_MOCK"
  });
  state = reduceOnboardingMock(state, {type: "CONFIGURE_PROFILE", profileId: "PLAYER"});
  state = reduceOnboardingMock(state, {type: "START_CONNECTION_TEST"});
  state = reduceOnboardingMock(state, {type: "COMPLETE_CONNECTION_TEST"});
  state = reduceOnboardingMock(state, {type: "MARK_LAUNCH_UNAVAILABLE"});

  assert.equal(state.accountMode, "existing");
  assert.equal(state.accountId, "190123");
  assert.equal(state.launchUnavailable, true);
  assert.equal("password" in state, false);

  state = reduceOnboardingMock(state, {type: "RESET"});
  assert.deepEqual(state, createOnboardingMockState());
});

test("new registration validates fields without returning a password", () => {
  assert.deepEqual(
    validateOnboardingMockAccount({mode: "new", displayName: "", password: "secret", passwordConfirmation: "secret", termsAccepted: true}),
    {valid: false, error: "displayNameRequired"}
  );
  assert.equal(validateOnboardingMockAccount({mode: "new", displayName: "PLAYER", password: "short", passwordConfirmation: "short", termsAccepted: true}).error, "passwordTooShort");
  assert.equal(validateOnboardingMockAccount({mode: "new", displayName: "PLAYER", password: "secret", passwordConfirmation: "different", termsAccepted: true}).error, "passwordMismatch");
  assert.equal(validateOnboardingMockAccount({mode: "new", displayName: "PLAYER", password: "secret", passwordConfirmation: "secret", termsAccepted: false}).error, "termsRequired");

  const valid = validateOnboardingMockAccount({mode: "new", displayName: " PLAYER ", password: "secret", passwordConfirmation: "secret", termsAccepted: true});
  assert.deepEqual(valid, {valid: true, accountId: "190000", accountName: "PLAYER"});
  assert.equal("password" in valid, false);
});

test("existing login validates the BMS-IR ID and password", () => {
  assert.equal(validateOnboardingMockAccount({mode: "existing", playerId: "189999", password: "secret"}).error, "playerIdInvalid");
  assert.equal(validateOnboardingMockAccount({mode: "existing", playerId: "190123", password: ""}).error, "passwordRequired");
  assert.deepEqual(
    validateOnboardingMockAccount({mode: "existing", playerId: "190123", password: "secret"}),
    {valid: true, accountId: "190123", accountName: "BMSIR_MOCK"}
  );
});
