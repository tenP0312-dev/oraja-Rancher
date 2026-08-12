import assert from "node:assert/strict";
import test from "node:test";

import {
  completedOnboardingMockSteps,
  createOnboardingMockState,
  onboardingMockStepState,
  reduceOnboardingMock
} from "./onboarding-mock.mjs";

test("the mock advances only through the ordered onboarding flow", () => {
  let state = createOnboardingMockState();
  assert.equal(state.stage, "account");
  assert.equal(completedOnboardingMockSteps(state), 0);

  state = reduceOnboardingMock(state, {type: "CONFIGURE_PROFILE", profileId: "BMSIR"});
  assert.equal(state.stage, "account");

  state = reduceOnboardingMock(state, {type: "START_ACCOUNT_LINK", method: "new"});
  assert.equal(state.linkMethod, "new");
  assert.equal(state.linkPending, true);

  state = reduceOnboardingMock(state, {type: "COMPLETE_ACCOUNT_LINK"});
  assert.equal(state.stage, "profile");
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
  state = reduceOnboardingMock(state, {type: "START_ACCOUNT_LINK", method: "existing"});
  state = reduceOnboardingMock(state, {type: "COMPLETE_ACCOUNT_LINK"});
  state = reduceOnboardingMock(state, {type: "CONFIGURE_PROFILE", profileId: "PLAYER"});
  state = reduceOnboardingMock(state, {type: "START_CONNECTION_TEST"});
  state = reduceOnboardingMock(state, {type: "COMPLETE_CONNECTION_TEST"});
  state = reduceOnboardingMock(state, {type: "MARK_LAUNCH_UNAVAILABLE"});

  assert.equal(state.linkMethod, "existing");
  assert.equal(state.launchUnavailable, true);

  state = reduceOnboardingMock(state, {type: "RESET"});
  assert.deepEqual(state, createOnboardingMockState());
});
