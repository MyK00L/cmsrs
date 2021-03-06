mod my_score;
mod score;
use score::ScoreTrait;

type ProtoScore = protos::common::Score;
type Score = my_score::MyScore;

fn transpose<T>(v: Vec<Vec<T>>) -> Vec<Vec<T>> {
    assert!(!v.is_empty());
    let len = v[0].len();
    let mut iters: Vec<_> = v.into_iter().map(|n| n.into_iter()).collect();
    (0..len)
        .map(|_| {
            iters
                .iter_mut()
                .map(|n| n.next().unwrap())
                .collect::<Vec<T>>()
        })
        .collect()
}

pub fn calc_subtask_score(
    testcases: &[protos::evaluation::TestcaseResult],
    opts: &protos::scoring::Subtask,
) -> ProtoScore {
    let method = protos::scoring::subtask::Method::from_i32(opts.method).unwrap();
    let testcase_scores = testcases.iter().map(|x| Score::from(x.score.clone()));
    let mut ans = match method {
        protos::scoring::subtask::Method::Min => testcase_scores.min().unwrap_or_else(Score::zero),
        protos::scoring::subtask::Method::Sum => testcase_scores.sum(),
    };
    let max_score = match method {
        protos::scoring::subtask::Method::Min => Score::one(),
        protos::scoring::subtask::Method::Sum => Score::from(ProtoScore {
            score: testcases.len() as f64,
        }), // TODO: better (maybe mul for usize for Score)
    };
    ans.rescale(max_score, Score::from(opts.max_score.clone()));
    ans.into()
}

pub fn calc_submission_score(
    subtasks: &[protos::evaluation::SubtaskResult],
    _opts: &protos::scoring::Problem,
) -> ProtoScore {
    //let method = protos::scoring::problem::Method::from_i32(opts.method).unwrap();

    subtasks
        .iter()
        .map(|x| Score::from(x.score.clone()))
        .sum::<Score>()
        .into()
}

pub fn calc_problem_score(
    submissions: &[protos::evaluation::EvaluationResult],
    opts: &protos::scoring::Problem,
) -> ProtoScore {
    let method = protos::scoring::problem::Method::from_i32(opts.method).unwrap();

    let submission_scores: Vec<Vec<Score>> = submissions
        .iter()
        .filter(|sub| !sub.subtask_results.is_empty())
        .map(|sub| {
            sub.subtask_results
                .iter()
                .map(|st| Score::from(st.score.clone()))
                .collect()
        })
        .collect();

    match method {
        protos::scoring::problem::Method::SumMax => transpose(submission_scores)
            .into_iter()
            .map(|subtask_scores| {
                subtask_scores
                    .into_iter()
                    .map(Score::from)
                    .max()
                    .unwrap_or_else(Score::zero)
            })
            .sum(),
        protos::scoring::problem::Method::MaxSum => submission_scores
            .into_iter()
            .map(|subtask_scores| subtask_scores.into_iter().map(Score::from).sum::<Score>())
            .max()
            .unwrap_or_else(Score::zero),
    }
    .into()
}

pub fn calc_user_score() -> ProtoScore {
    unimplemented!();
}
