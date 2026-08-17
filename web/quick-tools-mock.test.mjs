import test from "node:test";
import assert from "node:assert/strict";
import {QUICK_TOOL_KINDS, getQuickToolDefinition} from "./quick-tools-mock.mjs";

test("every quick tool has bilingual non-persistent mock content", () => {
  for (const language of ["ja", "en"]) {
    for (const kind of QUICK_TOOL_KINDS) {
      const definition = getQuickToolDefinition(language, kind);
      assert.ok(definition, `${language}/${kind}`);
      assert.equal(definition.persistent, false);
      assert.ok(definition.title.length > 0);
      assert.ok(definition.description.length > 0);
      assert.ok(definition.fields.length > 0);
      for (const field of definition.fields) {
        assert.match(field.type, /^(select|toggle)$/);
        assert.ok(field.label.length > 0);
        assert.ok(field.help.length > 0);
        if (field.type === "select") assert.ok(field.options.length > 1);
      }
    }
  }
});

test("unknown tools fail closed", () => {
  assert.equal(getQuickToolDefinition("ja", "unknown"), null);
});
