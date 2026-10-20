# Contributing to Agent Data Circuit (ADC)

## 1. Project Status

The Agent Data Circuit (ADC) is an **early draft protocol**. It is not a standard, nor is it finalized. There are no external audits, working groups, or official status claims.

The primary goal of this repository is to gather feedback to refine the specification. Your feedback is the point of this project.

## 2. Ways to Contribute

Contributions are prioritized by their impact on protocol robustness and clarity. Please follow this order of preference:

1.  **Build an independent conforming implementation**
    Implement ADC in a language other than Python. This is the most valuable contribution as it exposes ambiguities in the specification that the reference implementation may have missed.
2.  **Run conformance vectors and report failures**
    Execute the provided test vectors against your implementation or the reference server. Report any discrepancies as issues.
3.  **Improve the Python reference server/validator**
    Fix bugs, improve error messages, or optimize the `reference/` code.
4.  **Sharpen the specification**
    Open an issue pointing to the exact section of the spec that is ambiguous or incorrect. You must provide a concrete counterexample or a specific scenario that breaks the current wording.
5.  **Add examples for other verticals**
    Extend the `examples/` directory with use cases beyond e-commerce (e.g., documents, government services).

## 3. Development Setup

The reference server and validator rely on the Python standard library only.

*   **Requirements:** Python 3.11+
*   **Validation:**
    ```bash
    python reference/validator/adc.py validate examples/ecommerce
    ```
*   **Testing:**
    ```bash
    python -m pytest tests -q
    ```

## 4. Specification Change Process

Because ADC is a protocol, specification changes are governed by Request for Comments (RFCs). All spec changes must be proposed via a numbered Markdown file in the `rfcs/` directory.

An RFC must contain:
*   **Motivation:** Why the change is necessary.
*   **Spec Delta:** The exact text changes to the specification.
*   **Backward-Compatibility Impact:** How this affects existing implementations.
*   **Test Vectors:** New or updated vectors that verify the change.

The following initial RFCs serve as examples of the format:
*   `0001-discovery.md`
*   `0002-anchor-arithmetic.md`
*   `0003-server-side-aggregations.md`
*   `0004-dynamic-degradation.md`

## 5. Style Guidelines

*   **Normative Keywords:** Use keywords as defined in [RFC 2119](https://www.ietf.org/rfc/rfc2119).
*   **Verifiability:** Every `MUST` requirement in the specification MUST have a corresponding test vector. Do not add a `MUST` without a way to verify it.
*   **Precision:** Avoid vague language. Define terms explicitly.

## 6. Commit and PR Expectations

*   **Small and Focused:** Each Pull Request should address a single concern.
*   **Reference Spec Sections:** In your PR description, cite the specific section of the specification that your change affects.
*   **No Mixed Changes:** Do not mix formatting churn (whitespace, line endings) with semantic changes in the same commit.

## 7. Licensing

*   **Code:** Contributions to code are licensed under **Apache-2.0**.
*   **Documents:** Contributions to documentation and specification drafts are licensed under **CC BY 4.0**.

By contributing to this repository, you agree to license your contributions under the respective terms above.