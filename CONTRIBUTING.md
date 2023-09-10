# Contribution Guidelines

Thank you for considering contributing to our open source project! 
We appreciate your interest and value your input. 
This document outlines the process for contributing to the project, including reporting bugs, suggesting enhancements, 
and submitting pull requests.

## Table of Contents

- [Reporting Bugs](#reporting-bugs)
- [Suggesting Enhancements](#suggesting-enhancements)
- [Pull Request Process](#pull-request-process)
    - [Development pre-requisites](#development-pre-requisites)
    - [Making Changes](#making-changes)
    - [Review Process](#review-process)
- [Coding Rules](#coding-rules)
- [Commit Message Guidelines](#commit-message-guidelines)
- [License](#license)
- [Additional Resources](#additional-resources)

## Reporting Bugs

If you find a bug or issue, please follow these steps:

1. Check the [issue tracker](https://github.com/LNSD/rust-libp2p-pubsub/issues) to ensure the issue hasn't already been
   reported.
2. If it hasn't been reported, create a new issue, including a clear and descriptive title, as well as detailed steps
   to reproduce the issue if applicable.
3. If you can, include screenshots or error messages to help us understand the problem better and any relevant error
   messages or logs.

## Suggesting Enhancements

We welcome suggestions for improvements and new features. If you have an idea for a new feature or enhancement, please
follow these steps:

1. Check the [issue tracker](https://github.com/LNSD/rust-libp2p-pubsub/issues) to make sure it hasn't been suggested
   before.
2. If it's a new idea, create a new issue with a clear and descriptive title and a detailed description of the feature
   or enhancement you're proposing. Clearly describe the enhancement, including the motivation for the change and any potential benefits or use cases.
  
## Pull Request Process

### Development pre-requisites

To contribute to this project, you will need to install the following tools:

- Rust toolchain
- Cargo nextest (optional):
    ```
    cargo install cargo-nextest --locked
    ```

To work on the protocol buffer definitions, you will need to install the following tools:

- Buf CLI (https://docs.buf.build/installation)
- Protoc (https://grpc.io/docs/protoc-installation/)
- Protoc prost plugins (https://github.com/neoeinstein/protoc-gen-prost)
    ```
    cargo install protoc-gen-prost --locked
    cargo install protoc-gen-prost-crate --locked
    ```

### Making Changes

1. **Fork the repository**: To contribute to this project, start by forking the repository on GitHub. This creates a copy of the repository under your GitHub account, allowing you to experiment with changes without affecting the original project.
2. **Clone your fork**: After forking the repository, clone your fork to your local machine using `git clone`.
3. **Create a new branch**: Before making changes, create a new branch and switch to it using `git checkout -b BRANCH_NAME`. This ensures that your changes are isolated from the main branch, making it easier to contribute to the project again in the future.
4. **Make your changes**: Modify the code as needed, following the project's coding style and conventions. See the 
    [Coding Rules](#coding-rules) section for more information.
5. **Commit your changes**: Once you have made your changes, stage and commit them using `git add` and `git commit`. 
    Write a clear and concise commit message describing the changes you made. See the 
    [Commit Message Guidelines](#commit-message-guidelines) section for more information.
6. **Push your changes**: Push your changes to your fork on GitHub using `git push`.
7. **Create a pull request**: To submit your changes for review, create a pull request from your fork to the original 
    repository. In the description, provide a detailed explanation of your changes, why they are necessary, and how to 
    test them.

### Review Process

1. Ensure that your changes build and pass all tests before submitting a pull request.
2. Update the documentation, if applicable, to reflect your changes.
3. Your pull request will be reviewed by the project maintainers. They may request changes or provide feedback on your 
    contribution. Address any requested changes and resubmit the pull request if necessary.
4. Once your pull request is approved, it will be merged into the `main` branch.

## Coding Rules

To maintain code consistency and quality in our Rust project, please adhere to the following rules:

1. **Test Coverage**: All new features, bug fixes, or changes must be accompanied by one or more unit tests or 
    integration tests to ensure proper functionality and prevent regressions.

2. **Documentation**: Ensure that all public API methods, structures, and types are well-documented using Rustdoc 
    comments. This documentation should explain the purpose, usage, and any relevant examples for each item.

3. **Code Style**: We follow the official Rust coding style guidelines. In particular:
    - Use four spaces for indentation.
    - Keep lines of code under 100 characters in length to enhance code readability.
    - Maintain consistent naming conventions for variables, functions, and modules (e.g., snake_case for functions and 
       variables, CamelCase for types).
    - Avoid unnecessary or commented-out code. Remove or comment such code sparingly.
   
4. **Error Handling**: Rust's Result and Option types should be used for error handling where appropriate. Ensure that 
    error messages are informative and helpful for debugging.

5. **Unsafe Code**: Use `unsafe` blocks and operations with caution. Always provide clear comments explaining the 
    rationale for using unsafe code and ensure it's necessary for performance or interoperability reasons.

6. **Dependencies**: When adding new dependencies, consider their size, maintenance status, and compatibility with our 
    project. Keep dependencies up-to-date to benefit from bug fixes and security updates.

7. **Commit Atomicity**: Commits should be atomic and focus on a single concern or change. Whenever possible, a commit 
    should affect only one package/module within the project. This helps maintain a clear and traceable history of 
    changes.

8. **Code Formatting**: Use Rust's built-in code formatting tool (e.g., `rustfmt`) to ensure consistent code formatting 
    across the project. Run `cargo fmt` before committing your changes to ensure your code is formatted correctly.

By following these coding rules, we can maintain a high-quality Rust codebase that is easy to read, maintain, and 
collaborate on. Your contributions will help us create a more robust and reliable project.

## Commit Message Guidelines

We follow Angular's [conventional commit format](https://www.conventionalcommits.org/en/v1.0.0/) for our commits. Please
ensure that your commit messages follow this format:

```text
<type>(<scope>): <subject>
```

Where `<type>` must be one of the following:

* **chore:** Changes to the build process or auxiliary tools and libraries such as documentation generation or changes 
    that do not affect the meaning of the code in any way (e.g. `chore(ci): update github actions configuration` or 
    `chore(deps): update dependencies`)
* **docs:** Documentation only changes.
* **feat:** A new feature.
* **fix:** A bug fix.
* **refactor:** A code change that neither fixes a bug nor adds a feature.
* **test:** Adding missing tests or correcting existing tests

And `<scope>` should be the name of the package or module affected by the change. For example, `pubsub-core` or 
`floodsub`. If the change affects more than one package or module, do not specify a scope.

The `<subject>` contains a succinct description of the change:

* Use the imperative, present tense: "change" not "changed" nor "changes".
* Don't capitalize the first letter.
* No dot (.) at the end.

For example:

- `feat(floodsub): add support for message deduplication`
- `fix(pubsub-core): remove offending condition from validation logic`
- `docs: update README with new examples`
- `chore(deps): update dependencies`

This format helps us generate changelogs and understand the purpose of each commit.

## License

This project is licensed under the Apache 2.0 License. See the [LICENSE](LICENSE) file for more information.

## Additional Resources

- [Rust Documentation](https://doc.rust-lang.org/)
- [Cargo Documentation](https://doc.rust-lang.org/cargo/)

<hr>
Thank you for your interest in contributing to our Rust open source project. We look forward to collaborating with you 
and building a better project together!

