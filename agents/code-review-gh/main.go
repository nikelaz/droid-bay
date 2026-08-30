package main

import (
	"bytes"
	"context"
	_ "embed"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/nikelaz/droid-bay/helpers"
	"github.com/nikelaz/droid-bay/sdk"
)

const (
	TEMPERATURE    = 0.2
	MAX_STEPS      = 300
	RUN_TIMEOUT    = 60 * time.Minute
	CODEBASES_ROOT = ".codebases"
	LOGS_DIR       = ".logs"
	MAX_DIFF_BYTES = 300_000
	USER_AGENT     = "droid-bay-code-review-gh"
	GHAPI          = "https://api.github.com"
)

//go:embed system-prompt.md
var systemPrompt string

//go:embed user-prompt.md
var userPrompt string

//go:embed model-defaults.json
var modelDefaults []byte

type options struct {
	owner    string
	repo     string
	pr       int
	commit   string
	path     string
	model    string
	codebase string
	post     bool
	kind     string
}

func parseArgs() options {
	owner := flag.String("owner", "", "repository owner (user or org); required with -pr and -commit")
	repo := flag.String("repo", "", "repository name; required with -pr and -commit")
	num := flag.Int("pr", 0, "review pull request number")
	sha := flag.String("commit", "", "review a single commit (full or abbreviated SHA)")
	dir := flag.String("path", "", "review local unstaged changes in a git working tree")
	model := flag.String("model", "", "model to use; overrides model-defaults.json and skips its reasoning effort")
	codebase := flag.String("codebase", "", "path to a local checkout already at the target ref (skips cloning; GitHub modes only)")
	post := flag.Bool("post", true, "post the review to GitHub as a comment (GitHub modes only; -post=false prints the review instead)")
	flag.Parse()

	return options{
		owner:    *owner,
		repo:     *repo,
		pr:       *num,
		commit:   *sha,
		path:     *dir,
		model:    *model,
		codebase: *codebase,
		post:     *post,
	}
}

func (o *options) resolveTarget() error {
	chosen := 0
	if o.pr > 0 {
		chosen++
		o.kind = "pr"
	}
	if o.commit != "" {
		chosen++
		o.kind = "commit"
	}
	if o.path != "" {
		chosen++
		o.kind = "local"
	}
	if chosen != 1 {
		return fmt.Errorf("usage: code-review-gh requires exactly one target: -pr <number>, -commit <sha>, or -path <dir>")
	}
	if o.kind == "local" && o.codebase != "" {
		return fmt.Errorf("-codebase can only be used with -pr and -commit; for local reviews the working tree itself is used")
	}
	if o.kind != "local" && (o.owner == "" || o.repo == "") {
		return fmt.Errorf("-owner and -repo are required with -pr and -commit")
	}
	return nil
}

type reviewInput struct {
	target   string
	title    string
	meta     string
	desc     string
	diff     string
	codebase string
	ref      string
}

func main() {
	opts := parseArgs()

	log.SetFlags(0)

	closeLog, err := helpers.SetupRunLog(LOGS_DIR)
	if err != nil {
		log.Printf("continuing without a run log: %v", err)
	} else {
		defer closeLog()
	}

	if err := opts.resolveTarget(); err != nil {
		log.Fatal(err)
	}

	missing := sdk.MissingLLMEnv()
	if opts.kind != "local" {
		missing = append(missing, helpers.MissingEnv("GITHUB_TOKEN")...)
	}
	if len(missing) > 0 {
		log.Fatalf("missing environment variables: %s (set them in your shell environment)", strings.Join(missing, ", "))
	}

	token := ""
	if opts.kind != "local" {
		token = os.Getenv("GITHUB_TOKEN")
	}

	ctx, cancel := context.WithTimeout(context.Background(), RUN_TIMEOUT)
	defer cancel()

	cfg, err := sdk.ConfigForRun(modelDefaults, opts.model)
	if err != nil {
		log.Fatal(err)
	}

	var input *reviewInput
	switch opts.kind {
	case "pr":
		input, err = preparePR(ctx, token, opts)
	case "commit":
		input, err = prepareCommit(ctx, token, opts)
	case "local":
		input, err = prepareLocal(opts)
	}
	if err != nil {
		log.Fatal(err)
	}
	log.Printf("codebase at %s", input.codebase)

	codeClient := connectCodebaseMCP(ctx, input.codebase)
	defer codeClient.Close()

	codeTools, err := codeClient.Tools(ctx)
	if err != nil {
		log.Fatal(err)
	}

	prompt, err := renderPrompt(userPrompt, input)
	if err != nil {
		log.Fatal(err)
	}

	log.Printf("generating code review (max %d steps)", MAX_STEPS)
	review, err := sdk.Generate(ctx, cfg, systemPrompt, prompt,
		sdk.WithTemperature(TEMPERATURE),
		sdk.WithTools(codeTools...),
		sdk.WithMaxSteps(MAX_STEPS),
		sdk.WithDebugLog(log.Default()),
	)
	if err != nil {
		log.Fatal(err)
	}

	if strings.TrimSpace(review) == "" {
		log.Fatal("LLM returned an empty review; nothing to post")
	}

	log.Printf("\n--- code review (%d bytes) ---\n%s", len(review), review)

	switch opts.kind {
	case "local":
		fmt.Println(review)
	case "pr":
		if !opts.post {
			fmt.Println(review)
			return
		}
		if err := postReview(ctx, token, opts.owner, opts.repo, opts.pr, review); err != nil {
			log.Fatal(err)
		}
		log.Printf("review posted to %s/%s#%d", opts.owner, opts.repo, opts.pr)
	case "commit":
		if !opts.post {
			fmt.Println(review)
			return
		}
		if err := postCommitComment(ctx, token, opts.owner, opts.repo, input.ref, review); err != nil {
			log.Fatal(err)
		}
		log.Printf("review posted to %s/%s commit %s", opts.owner, opts.repo, shortSHA(input.ref))
	}
}

func preparePR(ctx context.Context, token string, opts options) (*reviewInput, error) {
	pr, err := fetchPullRequest(ctx, token, opts.owner, opts.repo, opts.pr)
	if err != nil {
		return nil, err
	}
	log.Printf("PR #%d: %s (state=%s, %s -> %s)", pr.Number, pr.Title, pr.State, pr.Base.Label, pr.Head.Label)

	files, err := fetchPRFiles(ctx, token, opts.owner, opts.repo, opts.pr)
	if err != nil {
		return nil, err
	}
	log.Printf("changed files: %d (+%d -%d)", len(files), pr.Additions, pr.Deletions)

	codebase := opts.codebase
	if codebase == "" {
		codebase, err = preparePRCodebase(opts.owner, opts.repo, opts.pr, CODEBASES_ROOT)
		if err != nil {
			return nil, err
		}
	}

	description := strings.TrimSpace(pr.Body)
	if description == "" {
		description = "(no description provided)"
	}

	return &reviewInput{
		target:   "Pull request #" + strconv.Itoa(pr.Number),
		title:    pr.Title,
		meta:     prMetadata(pr),
		desc:     description,
		diff:     renderDiff(files),
		codebase: codebase,
		ref:      pr.Head.SHA,
	}, nil
}

func prepareCommit(ctx context.Context, token string, opts options) (*reviewInput, error) {
	c, err := fetchCommit(ctx, token, opts.owner, opts.repo, opts.commit)
	if err != nil {
		return nil, err
	}
	log.Printf("commit %s: %s (+%d -%d)", shortSHA(c.SHA), commitSubject(c), c.Stats.Additions, c.Stats.Deletions)

	files := commitFiles(c)
	if len(files) == 0 {
		return nil, fmt.Errorf("commit %s reports no changed files", c.SHA)
	}

	codebase := opts.codebase
	if codebase == "" {
		codebase, err = prepareCommitCodebase(opts.owner, opts.repo, c.SHA, CODEBASES_ROOT)
		if err != nil {
			return nil, err
		}
	}

	description := strings.TrimSpace(c.Commit.Message)
	if description == "" {
		description = "(no description provided)"
	}

	return &reviewInput{
		target:   "Commit " + shortSHA(c.SHA),
		title:    commitSubject(c),
		meta:     commitMetadata(c),
		desc:     description,
		diff:     renderDiff(files),
		codebase: codebase,
		ref:      c.SHA,
	}, nil
}

func prepareLocal(opts options) (*reviewInput, error) {
	abs, err := filepath.Abs(opts.path)
	if err != nil {
		return nil, err
	}

	raw, err := gitDiff(abs)
	if err != nil {
		return nil, err
	}

	files := parseGitDiff(raw)
	if len(files) == 0 {
		return nil, fmt.Errorf("no unstaged changes in %s (git diff is empty)", abs)
	}

	totalAdd, totalDel := 0, 0
	for _, f := range files {
		totalAdd += f.Additions
		totalDel += f.Deletions
	}
	log.Printf("changed files: %d (+%d -%d)", len(files), totalAdd, totalDel)

	return &reviewInput{
		target:   "Local working tree",
		title:    "Unstaged changes in " + abs,
		meta:     localMetadata(abs, files),
		desc:     "Uncommitted, unstaged changes in the working tree (" + strconv.Itoa(len(files)) + " files, +" + strconv.Itoa(totalAdd) + "/-" + strconv.Itoa(totalDel) + " lines).\n\n" + gitStatusShort(abs),
		codebase: abs,
	}, nil
}

func commitSubject(c *Commit) string {
	subject := strings.TrimSpace(c.Commit.Message)
	if i := strings.IndexByte(subject, '\n'); i >= 0 {
		subject = subject[:i]
	}
	if subject == "" {
		subject = "(no commit message)"
	}
	return subject
}

func commitFiles(c *Commit) []changeFile {
	files := make([]changeFile, 0, len(c.Files))
	for _, f := range c.Files {
		files = append(files, changeFile{
			FileName:  f.FileName,
			Status:    f.Status,
			Additions: f.Additions,
			Deletions: f.Deletions,
			Changes:   f.Changes,
			Patch:     f.Patch,
		})
	}
	return files
}

func commitMetadata(c *Commit) string {
	return strings.Join([]string{
		"- SHA: " + c.SHA,
		"- Author: " + c.Commit.Author.Name,
		"- Date: " + c.Commit.Author.Date,
		"- Parents: " + strconv.Itoa(len(c.Parents)),
		"- Stats: " + strconv.Itoa(len(c.Files)) + " files, +" + strconv.Itoa(c.Stats.Additions) + "/-" + strconv.Itoa(c.Stats.Deletions) + " lines",
		"- URL: " + c.HTMLURL,
	}, "\n")
}

func prMetadata(pr *PullRequest) string {
	return strings.Join([]string{
		"- Author: @" + pr.User.Login,
		"- State: " + pr.State,
		"- Branch: " + pr.Base.Label + " <- " + pr.Head.Label,
		"- Head SHA: " + pr.Head.SHA,
		"- Stats: " + strconv.Itoa(pr.ChangedFiles) + " files, +" + strconv.Itoa(pr.Additions) + "/-" + strconv.Itoa(pr.Deletions) + " lines",
		"- URL: " + pr.HTMLURL,
		"- Created: " + pr.CreatedAt,
		"- Updated: " + pr.UpdatedAt,
	}, "\n")
}

func localMetadata(dir string, files []changeFile) string {
	totalAdd, totalDel := 0, 0
	for _, f := range files {
		totalAdd += f.Additions
		totalDel += f.Deletions
	}
	return strings.Join([]string{
		"- Path: " + dir,
		"- Branch: " + gitOutput(dir, "rev-parse", "--abbrev-ref", "HEAD"),
		"- HEAD: " + gitOutput(dir, "log", "-1", "--format=%h %s"),
		"- Stats: " + strconv.Itoa(len(files)) + " files, +" + strconv.Itoa(totalAdd) + "/-" + strconv.Itoa(totalDel) + " lines",
	}, "\n")
}

func renderPrompt(path string, in *reviewInput) (string, error) {
	tpl, err := os.ReadFile(path)
	if err != nil {
		return "", fmt.Errorf("read user prompt template: %w", err)
	}
	return strings.NewReplacer(
		"{{target}}", in.target,
		"{{title}}", in.title,
		"{{metadata}}", in.meta,
		"{{description}}", in.desc,
		"{{diff}}", in.diff,
	).Replace(string(tpl)), nil
}

func renderDiff(files []changeFile) string {
	sorted := make([]changeFile, len(files))
	copy(sorted, files)
	sort.Slice(sorted, func(i, j int) bool {
		a := sorted[i].Additions + sorted[i].Deletions
		b := sorted[j].Additions + sorted[j].Deletions
		if a == b {
			return sorted[i].FileName < sorted[j].FileName
		}
		return a > b
	})

	var b strings.Builder
	budget := MAX_DIFF_BYTES
	for i, f := range sorted {
		status := f.Status
		if status == "" {
			status = "changed"
		}
		fmt.Fprintf(&b, "%d. `%s` (%s, +%d -%d)\n\n", i+1, f.FileName, status, f.Additions, f.Deletions)

		patch := strings.TrimSpace(f.Patch)
		if patch == "" {
			b.WriteString("_no patch available (binary, submodule, or generated file)_\n\n")
			continue
		}

		code := "```diff\n" + patch + "\n```"
		if budget-len(code) < 0 {
			fmt.Fprintf(&b, "_patch omitted (diff budget exhausted) — read `%s` with the codebase tools if needed_\n\n", f.FileName)
			continue
		}
		b.WriteString(code)
		b.WriteString("\n\n")
		budget -= len(code)
	}

	out := strings.TrimSpace(b.String())
	if out == "" {
		return "_no changed files_"
	}
	return out
}

func shortSHA(sha string) string {
	if len(sha) > 7 {
		return sha[:7]
	}
	return sha
}

func gitDiff(dir string) (string, error) {
	out, err := gitRun(dir, "diff")
	if err != nil {
		return "", fmt.Errorf("git diff in %s: %v: %s", dir, err, out)
	}
	return out, nil
}

func gitStatusShort(dir string) string {
	status := gitOutput(dir, "status", "--short")
	if status == "" {
		return "(clean status)"
	}
	return "```\n" + status + "\n```"
}

func gitOutput(dir string, args ...string) string {
	out, err := gitRun(dir, args...)
	if err != nil {
		return ""
	}
	return out
}

func gitRun(dir string, args ...string) (string, error) {
	cmd := exec.Command("git", args...)
	cmd.Dir = dir
	cmd.Env = append(os.Environ(), "GIT_TERMINAL_PROMPT=0")
	out, err := cmd.CombinedOutput()
	return strings.TrimSpace(string(out)), err
}

func parseGitDiff(raw string) []changeFile {
	if strings.TrimSpace(raw) == "" {
		return nil
	}
	sections := strings.Split(raw, "\ndiff --git ")
	var files []changeFile
	for i, sec := range sections {
		if i > 0 {
			sec = "diff --git " + sec
		}
		if !strings.HasPrefix(sec, "diff --git ") || strings.TrimSpace(sec) == "" {
			continue
		}
		files = append(files, parseDiffSection(sec))
	}
	return files
}

func parseDiffSection(sec string) changeFile {
	var f changeFile
	f.Patch = strings.TrimRight(sec, "\n")

	for _, l := range strings.Split(sec, "\n") {
		switch {
		case strings.HasPrefix(l, "new file mode"):
			f.Status = "added"
		case strings.HasPrefix(l, "deleted file mode"):
			f.Status = "removed"
		case strings.HasPrefix(l, "rename from"):
			f.Status = "renamed"
		case strings.HasPrefix(l, "+++ b/"):
			if f.FileName == "" {
				f.FileName = strings.TrimPrefix(l, "+++ b/")
			}
		case strings.HasPrefix(l, "diff --git "):
			if f.FileName == "" {
				rest := strings.TrimPrefix(l, "diff --git ")
				if _, after, ok := strings.Cut(rest, " b/"); ok {
					f.FileName = after
				}
			}
		case strings.HasPrefix(l, "+") && !strings.HasPrefix(l, "+++"):
			f.Additions++
		case strings.HasPrefix(l, "-") && !strings.HasPrefix(l, "---"):
			f.Deletions++
		}
	}

	if f.FileName == "" {
		f.FileName = "(unknown)"
	}
	if f.Status == "" {
		f.Status = "modified"
	}
	f.Changes = f.Additions + f.Deletions
	return f
}

type PRUser struct {
	Login string `json:"login"`
}

type PRRef struct {
	Label string `json:"label"`
	Ref   string `json:"ref"`
	SHA   string `json:"sha"`
}

type PullRequest struct {
	Number       int    `json:"number"`
	Title        string `json:"title"`
	Body         string `json:"body"`
	State        string `json:"state"`
	HTMLURL      string `json:"html_url"`
	CreatedAt    string `json:"created_at"`
	UpdatedAt    string `json:"updated_at"`
	User         PRUser `json:"user"`
	Base         PRRef  `json:"base"`
	Head         PRRef  `json:"head"`
	Additions    int    `json:"additions"`
	Deletions    int    `json:"deletions"`
	ChangedFiles int    `json:"changed_files"`
}

type Commit struct {
	SHA    string `json:"sha"`
	Commit struct {
		Author struct {
			Name string `json:"name"`
			Date string `json:"date"`
		} `json:"author"`
		Message string `json:"message"`
	} `json:"commit"`
	Parents []struct {
		SHA string `json:"sha"`
	} `json:"parents"`
	Stats struct {
		Additions int `json:"additions"`
		Deletions int `json:"deletions"`
		Total     int `json:"total"`
	} `json:"stats"`
	Files []struct {
		FileName  string `json:"filename"`
		Status    string `json:"status"`
		Additions int    `json:"additions"`
		Deletions int    `json:"deletions"`
		Changes   int    `json:"changes"`
		Patch     string `json:"patch"`
	} `json:"files"`
	HTMLURL string `json:"html_url"`
}

type changeFile struct {
	FileName  string
	Status    string
	Additions int
	Deletions int
	Changes   int
	Patch     string
}

func fetchPullRequest(ctx context.Context, token, owner, repo string, num int) (*PullRequest, error) {
	var pr PullRequest
	url := fmt.Sprintf("%s/repos/%s/%s/pulls/%d", GHAPI, owner, repo, num)
	if err := ghGet(ctx, token, url, &pr); err != nil {
		return nil, fmt.Errorf("fetch pull request: %w", err)
	}
	if pr.Number == 0 || pr.Title == "" {
		return nil, fmt.Errorf("no pull request found at %s", url)
	}
	return &pr, nil
}

func fetchPRFiles(ctx context.Context, token, owner, repo string, num int) ([]changeFile, error) {
	var files []changeFile
	for page := 1; page <= 10; page++ {
		var batch []PRFile
		url := fmt.Sprintf("%s/repos/%s/%s/pulls/%d/files?per_page=100&page=%d", GHAPI, owner, repo, num, page)
		if err := ghGet(ctx, token, url, &batch); err != nil {
			return nil, fmt.Errorf("fetch pull request files: %w", err)
		}
		for _, f := range batch {
			files = append(files, changeFile{
				FileName:  f.FileName,
				Status:    f.Status,
				Additions: f.Additions,
				Deletions: f.Deletions,
				Changes:   f.Changes,
				Patch:     f.Patch,
			})
		}
		if len(batch) < 100 {
			break
		}
	}
	return files, nil
}

type PRFile struct {
	FileName  string `json:"filename"`
	Status    string `json:"status"`
	Additions int    `json:"additions"`
	Deletions int    `json:"deletions"`
	Changes   int    `json:"changes"`
	Patch     string `json:"patch"`
}

func fetchCommit(ctx context.Context, token, owner, repo, sha string) (*Commit, error) {
	var c Commit
	url := fmt.Sprintf("%s/repos/%s/%s/commits/%s", GHAPI, owner, repo, sha)
	if err := ghGet(ctx, token, url, &c); err != nil {
		return nil, fmt.Errorf("fetch commit: %w", err)
	}
	if c.SHA == "" {
		return nil, fmt.Errorf("no commit found at %s", url)
	}
	return &c, nil
}

func ghGet(ctx context.Context, token, url string, v any) error {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return err
	}
	return ghDo(req, token, v)
}

func ghDo(req *http.Request, token string, v any) error {
	req.Header.Set("Authorization", "Bearer "+token)
	req.Header.Set("Accept", "application/vnd.github+json")
	req.Header.Set("X-GitHub-Api-Version", "2022-11-28")
	req.Header.Set("User-Agent", USER_AGENT)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		body, _ := io.ReadAll(io.LimitReader(resp.Body, 4096))
		return fmt.Errorf("GitHub API %s: %s", resp.Status, strings.TrimSpace(string(body)))
	}
	return json.NewDecoder(resp.Body).Decode(v)
}

func postReview(ctx context.Context, token, owner, repo string, num int, body string) error {
	payload, err := json.Marshal(map[string]any{
		"body":  body,
		"event": "COMMENT",
	})
	if err != nil {
		return err
	}
	url := fmt.Sprintf("%s/repos/%s/%s/pulls/%d/reviews", GHAPI, owner, repo, num)
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, url, bytes.NewReader(payload))
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/json")
	var res map[string]any
	if err := ghDo(req, token, &res); err != nil {
		return fmt.Errorf("post review: %w", err)
	}
	return nil
}

func postCommitComment(ctx context.Context, token, owner, repo, sha, body string) error {
	payload, err := json.Marshal(map[string]any{"body": body})
	if err != nil {
		return err
	}
	url := fmt.Sprintf("%s/repos/%s/%s/commits/%s/comments", GHAPI, owner, repo, sha)
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, url, bytes.NewReader(payload))
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/json")
	var res map[string]any
	if err := ghDo(req, token, &res); err != nil {
		return fmt.Errorf("post commit comment: %w", err)
	}
	return nil
}

func preparePRCodebase(owner, repo string, num int, root string) (string, error) {
	dir, err := helpers.CloneRepo(owner, repo, root)
	if err != nil {
		return "", err
	}

	branch := "pr-" + strconv.Itoa(num)
	for _, args := range [][]string{
		{"fetch", "origin", fmt.Sprintf("pull/%d/head:%s", num, branch)},
		{"checkout", "-f", branch},
	} {
		if out, err := gitRun(dir, args...); err != nil {
			return "", fmt.Errorf("git %s: %v: %s", strings.Join(args, " "), err, out)
		}
	}
	return dir, nil
}

func prepareCommitCodebase(owner, repo, sha, root string) (string, error) {
	dir, err := helpers.CloneRepo(owner, repo, root)
	if err != nil {
		return "", err
	}

	for _, args := range [][]string{
		{"fetch", "--depth", "1", "origin", sha},
		{"checkout", "-f", sha},
	} {
		if out, err := gitRun(dir, args...); err != nil {
			return "", fmt.Errorf("git %s: %v: %s", strings.Join(args, " "), err, out)
		}
	}
	return dir, nil
}

func connectCodebaseMCP(ctx context.Context, root string) *sdk.MCPClient {
	var argv []string
	if cmd := os.Getenv("CODEBASE_MCP_CMD"); cmd != "" {
		var err error
		argv, err = helpers.SplitArgs(cmd)
		if err != nil {
			log.Fatalf("parse CODEBASE_MCP_CMD: %v", err)
		}
	} else {
		argv = []string{"npx", "-y", "@modelcontextprotocol/server-filesystem", root}
	}

	if len(argv) == 0 {
		log.Fatal("CODEBASE_MCP_CMD is empty; expected at least a command")
	}

	client, err := sdk.ConnectStdio(ctx, "code-review-gh-codebase", argv[0], argv[1:], nil)
	if err != nil {
		log.Fatalf("connect to codebase MCP server: %v (is it installed / Node available?)", err)
	}

	return client
}