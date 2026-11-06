const { CLOUDFLARE_ACCOUNT_ID: account, CLOUDFLARE_API_TOKEN: token, GITHUB_TOKEN: githubToken, GITHUB_REPOSITORY: repo } = process.env;
const project = process.env.PAGES_PROJECT ?? 'untwist';
const keepProduction = Number(process.env.KEEP_PRODUCTION ?? 5);
const mainBuildBranch = 'main-latest';
const [mode, branchArg] = process.argv.slice(2);

if (!account || !token) {
  console.log('Cloudflare secrets are not set, nothing to clean up');
  process.exit(0);
}

const api = `https://api.cloudflare.com/client/v4/accounts/${account}/pages/projects/${project}`;

async function cloudflare(path, init = {}) {
  const response = await fetch(`${api}${path}`, {
    ...init,
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
  });
  const body = await response.json();
  if (!body.success) {
    throw new Error(`${init.method ?? 'GET'} ${path}: ${JSON.stringify(body.errors)}`);
  }
  return body;
}

async function deployments(env) {
  const all = [];
  for (let page = 1; ; page += 1) {
    const { result, result_info: info } = await cloudflare(`/deployments?env=${env}&page=${page}&per_page=25`);
    all.push(...result);
    if (!info || page * info.per_page >= info.total_count || result.length === 0) {
      return all;
    }
  }
}

function branchOf(deployment) {
  return deployment.deployment_trigger?.metadata?.branch ?? '';
}

async function remove(deployment, reason) {
  await cloudflare(`/deployments/${deployment.id}?force=true`, { method: 'DELETE' });
  console.log(`deleted ${deployment.short_id} (${branchOf(deployment) || 'no branch'}): ${reason}`);
}

async function liveBranches() {
  const names = new Set();
  for (let page = 1; ; page += 1) {
    const response = await fetch(`https://api.github.com/repos/${repo}/branches?per_page=100&page=${page}`, {
      headers: { Authorization: `Bearer ${githubToken}`, Accept: 'application/vnd.github+json' },
    });
    if (!response.ok) {
      throw new Error(`listing branches: ${response.status}`);
    }
    const batch = await response.json();
    batch.forEach((branch) => names.add(branch.name));
    if (batch.length < 100) {
      return names;
    }
  }
}

async function cleanBranch(branch) {
  const previews = (await deployments('preview')).filter((deployment) => branchOf(deployment) === branch);
  for (const deployment of previews) {
    await remove(deployment, `branch ${branch} closed`);
  }
  console.log(`${previews.length} preview deployment(s) removed for ${branch}`);
}

async function sweep() {
  const { result: details } = await cloudflare('');
  const liveId = details.canonical_deployment?.id;
  const production = (await deployments('production')).sort((a, b) => b.created_on.localeCompare(a.created_on));
  for (const deployment of production.slice(keepProduction)) {
    if (deployment.id !== liveId) {
      await remove(deployment, `older than the newest ${keepProduction} production builds`);
    }
  }
  const branches = await liveBranches();
  const previews = await deployments('preview');
  const mainBuilds = previews
    .filter((deployment) => branchOf(deployment) === mainBuildBranch)
    .sort((a, b) => b.created_on.localeCompare(a.created_on));
  for (const deployment of mainBuilds.slice(keepProduction)) {
    await remove(deployment, `older than the newest ${keepProduction} builds of main`);
  }
  for (const deployment of previews) {
    const branch = branchOf(deployment);
    if (branch === mainBuildBranch) {
      continue;
    }
    if (branch && !branches.has(branch)) {
      await remove(deployment, `branch ${branch} no longer exists`);
    } else if (branch && branch === details.production_branch) {
      await remove(deployment, `a preview of the production branch ${branch}`);
    }
  }
}

if (mode === 'branch') {
  if (!branchArg) {
    throw new Error('usage: pages-cleanup.mjs branch <name>');
  }
  await cleanBranch(branchArg);
} else {
  await sweep();
}
