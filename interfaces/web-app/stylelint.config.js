export default {
  extends: ["stylelint-config-standard", "stylelint-config-html"],
  rules: {
    "no-descending-specificity": true,
    "declaration-block-no-duplicate-properties": true,
    "unit-no-unknown": true,
    "font-family-no-missing-generic-family-keyword": true
  }
};
