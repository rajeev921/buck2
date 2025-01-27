/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use anyhow::Context as _;
use buck2_common::invocation_roots::find_invocation_roots;
use buck2_common::legacy_configs::cells::BuckConfigBasedCells;
use buck2_common::legacy_configs::cells::DaemonStartupConfig;
use buck2_core::cells::CellResolver;
use buck2_core::fs::paths::abs_norm_path::AbsNormPath;
use buck2_core::fs::paths::abs_norm_path::AbsNormPathBuf;
use buck2_core::fs::project::ProjectRoot;
use buck2_core::fs::working_dir::WorkingDir;
use once_cell::sync::OnceCell;

/// Lazy-computed immediate config data. This is produced by reading the root buckconfig (but not
/// processing any includes).
struct ImmediateConfigContextData {
    cell_resolver: CellResolver,
    daemon_startup_config: DaemonStartupConfig,
    project_filesystem: ProjectRoot,
}

pub struct ImmediateConfigContext<'a> {
    // Deliberately use `OnceCell` rather than `Lazy` because `Lazy` forces
    // us to have a shared reference to the underlying `anyhow::Error` which
    // we cannot use to correct chain the errors. Using `OnceCell` means
    // we don't get the result by a shared reference but instead as local
    // value which can be returned.
    data: OnceCell<ImmediateConfigContextData>,
    cwd: &'a WorkingDir,
    trace: Vec<AbsNormPathBuf>,
}

impl<'a> ImmediateConfigContext<'a> {
    pub fn new(cwd: &'a WorkingDir) -> Self {
        Self {
            data: OnceCell::new(),
            cwd,
            trace: Vec::new(),
        }
    }

    pub fn push_trace(&mut self, path: &AbsNormPath) {
        self.trace.push(path.to_buf());
    }

    pub fn trace(&self) -> &[AbsNormPathBuf] {
        &self.trace
    }

    pub fn daemon_startup_config(&self) -> anyhow::Result<&DaemonStartupConfig> {
        Ok(&self.data()?.daemon_startup_config)
    }

    /// Resolves an argument which can possibly be a cell-relative path.
    /// If the argument is not a cell-relative path, it returns `None`.
    /// Otherwise, it tries to resolve the cell and returns a `Result`.
    pub fn resolve_cell_path_arg(&self, path: &str) -> Option<anyhow::Result<AbsNormPathBuf>> {
        path.split_once("//")
            .map(|(cell_alias, cell_relative_path)| {
                self.resolve_cell_path(cell_alias, cell_relative_path)
            })
    }

    /// Resolves a cell path (i.e., contains `//`) into an absolute path. The cell path must have
    /// been split into two components: `cell_alias` and `cell_path`. For example, if the cell path
    /// is `cell//path/to/file`, then:
    ///   - `cell_alias` would be `cell`
    ///   - `cell_relative_path` would be `path/to/file`
    pub fn resolve_cell_path(
        &self,
        cell_alias: &str,
        cell_relative_path: &str,
    ) -> anyhow::Result<AbsNormPathBuf> {
        let data = self.data()?;

        data.cell_resolver.resolve_cell_relative_path(
            cell_alias,
            cell_relative_path,
            &data.project_filesystem,
            self.cwd.path(),
        )
    }

    fn data(&self) -> anyhow::Result<&ImmediateConfigContextData> {
        self.data
            .get_or_try_init(|| {
                let roots = find_invocation_roots(self.cwd.path())?;

                // See comment in `ImmediateConfig` about why we use `OnceCell` rather than `Lazy`
                let project_filesystem = roots.project_root;
                let cfg = BuckConfigBasedCells::parse_immediate_config(&project_filesystem)?;

                anyhow::Ok(ImmediateConfigContextData {
                    cell_resolver: cfg.cell_resolver,
                    daemon_startup_config: cfg.daemon_startup_config,
                    project_filesystem,
                })
            })
            .context("Error creating cell resolver")
    }
}
