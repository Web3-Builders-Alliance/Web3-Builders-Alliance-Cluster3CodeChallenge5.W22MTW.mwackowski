#![cfg(test)]

use cosmwasm_std::{to_binary, Addr, Empty, Uint128, WasmMsg, QuerierWrapper};
use cw20::{BalanceResponse, MinterResponse};
use cw20_base::msg::QueryMsg;
use cw3::Vote;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use cw_utils::{Duration, Threshold};

use crate::contract::{execute, instantiate, query};
use crate::msg::{ExecuteMsg, InstantiateMsg, Voter, QueryMsg as QueryMsgCw3};

use counter::{self, helpers::CounterContract};

fn mock_app() -> App {
    App::default()
}

pub fn contract_cw3_fixed_multisig() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query);
    Box::new(contract)
}

pub fn contract_cw20() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

pub fn contract_counter() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        counter::contract::execute,
        counter::contract::instantiate,
        counter::contract::query,
    );
    Box::new(contract)
}

fn store_code() -> (App, u64, u64) {
    let mut app = mock_app();
    let cw3_multisig = app.store_code(contract_cw3_fixed_multisig());
    let counter_id = app.store_code(contract_counter());
    (app, cw3_multisig, counter_id)
}

fn get_count(querier: QuerierWrapper, counter_addr: &Addr) -> i32 {
    let counter_value_query = counter::msg::QueryMsg::GetCount {};
    let count: counter::msg::GetCountResponse = querier
        .query_wasm_smart(counter_addr, &counter_value_query)
        .unwrap();
    count.count
}

#[test]
// cw3 multisig account can control cw20 admin actions
fn cw3_controls_cw20() {
    let mut router = mock_app();

    // setup cw3 multisig with 3 accounts
    let cw3_id = router.store_code(contract_cw3_fixed_multisig());

    let addr1 = Addr::unchecked("addr1");
    let addr2 = Addr::unchecked("addr2");
    let addr3 = Addr::unchecked("addr3");
    let cw3_instantiate_msg = InstantiateMsg {
        voters: vec![
            Voter {
                addr: addr1.to_string(),
                weight: 1,
            },
            Voter {
                addr: addr2.to_string(),
                weight: 1,
            },
            Voter {
                addr: addr3.to_string(),
                weight: 1,
            },
        ],
        threshold: Threshold::AbsoluteCount { weight: 2 },
        max_voting_period: Duration::Height(3),
    };

    let multisig_addr = router
        .instantiate_contract(
            cw3_id,
            addr1.clone(),
            &cw3_instantiate_msg,
            &[],
            "Consortium",
            None,
        )
        .unwrap();

    // setup cw20 as cw3 multisig admin
    let cw20_id = router.store_code(contract_cw20());

    let cw20_instantiate_msg = cw20_base::msg::InstantiateMsg {
        name: "Consortium Token".parse().unwrap(),
        symbol: "CST".parse().unwrap(),
        decimals: 6,
        initial_balances: vec![],
        mint: Some(MinterResponse {
            minter: multisig_addr.to_string(),
            cap: None,
        }),
        marketing: None,
    };
    let cw20_addr = router
        .instantiate_contract(
            cw20_id,
            multisig_addr.clone(),
            &cw20_instantiate_msg,
            &[],
            "Consortium",
            None,
        )
        .unwrap();

    // mint some cw20 tokens according to proposal result
    let mint_recipient = Addr::unchecked("recipient");
    let mint_amount = Uint128::new(1000);
    let cw20_mint_msg = cw20_base::msg::ExecuteMsg::Mint {
        recipient: mint_recipient.to_string(),
        amount: mint_amount,
    };

    let execute_mint_msg = WasmMsg::Execute {
        contract_addr: cw20_addr.to_string(),
        msg: to_binary(&cw20_mint_msg).unwrap(),
        funds: vec![],
    };
    let propose_msg = ExecuteMsg::Propose {
        title: "Mint tokens".to_string(),
        description: "Need to mint tokens".to_string(),
        msgs: vec![execute_mint_msg.into()],
        latest: None,
    };
    // propose mint
    router
        .execute_contract(addr1.clone(), multisig_addr.clone(), &propose_msg, &[])
        .unwrap();

    // second votes
    let vote2_msg = ExecuteMsg::Vote {
        proposal_id: 1,
        vote: Vote::Yes,
    };
    router
        .execute_contract(addr2, multisig_addr.clone(), &vote2_msg, &[])
        .unwrap();

    // only 1 vote and msg mint fails
    let execute_proposal_msg = ExecuteMsg::Execute { proposal_id: 1 };
    // execute mint
    router
        .execute_contract(addr1, multisig_addr, &execute_proposal_msg, &[])
        .unwrap();

    // check the mint is successful
    let cw20_balance_query = QueryMsg::Balance {
        address: mint_recipient.to_string(),
    };
    let balance: BalanceResponse = router
        .wrap()
        .query_wasm_smart(&cw20_addr, &cw20_balance_query)
        .unwrap();

    // compare minted amount
    assert_eq!(balance.balance, mint_amount);
}

#[test]
fn cw3_3_of_5_addr() {
    // let mut router = mock_app();

    // setup cw3 multisig with 5 accounts
    // let cw3_id = router.store_code(contract_cw3_fixed_multisig());
    let (mut app, cw3_multisig, counter_id) = store_code();
    

    let addr1 = Addr::unchecked("addr1");
    let addr2 = Addr::unchecked("addr2");
    let addr3 = Addr::unchecked("addr3");
    let addr4 = Addr::unchecked("addr4");
    let addr5 = Addr::unchecked("addr5");
    let cw3_instantiate_msg = InstantiateMsg {
        voters: vec![
            Voter {
                addr: addr1.to_string(),
                weight: 1,
            },
            Voter {
                addr: addr2.to_string(),
                weight: 1,
            },
            Voter {
                addr: addr3.to_string(),
                weight: 1,
            },
            Voter {
                addr: addr4.to_string(),
                weight: 1,
            },
            Voter {
                addr: addr5.to_string(),
                weight: 1,
            },
        ],
        threshold: Threshold::AbsoluteCount { weight: 3 },
        max_voting_period: Duration::Height(3),
    };

    let multisig_addr = app
        .instantiate_contract(
            cw3_multisig,
            addr1.clone(),
            &cw3_instantiate_msg,
            &[],
            "3 of 5 multisig",
            None,
        )
        .unwrap();

    // setup cw20 as cw3 multisig admin
    // let counter_id = router.store_code(contract_counter());

    let counter_instantiate_msg = counter::msg::InstantiateMsg {
        count: 0,
        owner: multisig_addr.to_string()
    };

    let counter_addr = app
        .instantiate_contract(
            counter_id,
            addr1.clone(),
            &counter_instantiate_msg,
            &[],
            "3 of 5 counter",
            None,
        )
        .unwrap();

    let proposal = app.execute_contract(
        addr1.clone(),
        multisig_addr.clone(),
        &ExecuteMsg::Propose {
            title: "Increment".to_string(),
            description: "Let's increment the counter!".to_string(),
            msgs: vec![WasmMsg::Execute {
                contract_addr: counter_addr.to_string(),
                msg: to_binary(&counter::msg::ExecuteMsg::Increment {}).unwrap(),
                funds: vec![],
            }
            .into()],
            latest: None,
        },
        &[],
    );
    assert!(proposal.is_ok());

    let addr2_votes_yes = app.execute_contract(
        addr2.clone(),
        multisig_addr.clone(),
        &ExecuteMsg::Vote {
            proposal_id: 1,
            vote: Vote::Yes,
        },
        &[],
    );
    assert!(addr2_votes_yes.is_ok());


    // only 2 votes and executing increment fails
    assert!(app
        .execute_contract(
            addr1.clone(),
            multisig_addr.clone(),
            &ExecuteMsg::Execute { proposal_id: 1 },
            &[],
        )
        .is_err());

    assert_eq!(get_count(app.wrap(), &counter_addr), 0);
    let addr3_votes_no = app.execute_contract(
        addr3.clone(),
        multisig_addr.clone(),
        &ExecuteMsg::Vote {
            proposal_id: 1,
            vote: Vote::No,
        },
        &[],
    );
    assert!(addr3_votes_no.is_ok());

    // only 2 yes votes, increment still fails
    assert!(app
        .execute_contract(
            addr1.clone(),
            multisig_addr.clone(),
            &ExecuteMsg::Execute { proposal_id: 1 },
            &[],
        )
        .is_err());
    assert_eq!(get_count(app.wrap(), &counter_addr), 0);

    let addr4_votes_yes = app.execute_contract(
        addr4,
        multisig_addr.clone(),
        &ExecuteMsg::Vote {
            proposal_id: 1,
            vote: Vote::Yes,
        },
        &[],
    );
    assert!(addr4_votes_yes.is_ok());

    // now 3 votes and msg increment passes!
    assert!(app
        .execute_contract(
            addr1.clone(),
            multisig_addr.clone(),
            &ExecuteMsg::Execute { proposal_id: 1 },
            &[],
        )
        .is_ok());

    //  Query the counter contract and assert that the count has incremented.
    assert_eq!(get_count(app.wrap(), &counter_addr), 1);

    //  Once that proposal has passed add a new proposal to the test to reset the counter and then vote on that as well
    let proposal_reset = app.execute_contract(
        addr1.clone(),
        multisig_addr.clone(),
        &ExecuteMsg::Propose {
            title: "Reset".to_string(),
            description: "Let's reset the counter now".to_string(),
            msgs: vec![WasmMsg::Execute {
                contract_addr: counter_addr.to_string(),
                msg: to_binary(&counter::msg::ExecuteMsg::Reset {count: 0}).unwrap(),
                funds: vec![],
            }
            .into()],
            latest: None,
        },
        &[],
    );
    assert!(proposal_reset.is_ok());

    // Let's see if the total amount of proposals is 2
    let msg = QueryMsgCw3::ListProposals{start_after: Some(0), limit: Some(10)};
    let res: cw3::ProposalListResponse = app
        .wrap()
        .query_wasm_smart(multisig_addr
        .to_string(), &msg)
        .unwrap();
    assert_eq!(res.proposals.len(), 2);
    assert_eq!(res.proposals[0].description, "Let's increment the counter!");
    assert_eq!(res.proposals[1].description, "Let's reset the counter now");
    println!("ListProposal query response: {:?}", res);

    // vote on reset
    let addr2_votes_yes_on_reset = app.execute_contract(
        addr2.clone(),
        multisig_addr.clone(),
        &ExecuteMsg::Vote {
            proposal_id: 2,
            vote: Vote::Yes,
        },
        &[],
    );
    assert!(addr2_votes_yes_on_reset.is_ok());

    // Let's see if both votes are set to "yes"
    let msg = QueryMsgCw3::ListVotes{proposal_id: 2, start_after: Some("0".to_string()), limit: Some(10)};
    let res: cw3::VoteListResponse = app
        .wrap()
        .query_wasm_smart(multisig_addr
        .to_string(), &msg)
        .unwrap();
    assert_eq!(res.votes[0].vote, Vote::Yes);
    assert_eq!(res.votes[1].vote, Vote::Yes);
    
    // last vote "Yes" on reset
    let addr3_votes_yes_on_reset = app
        .execute_contract(
        addr3.clone(),
        multisig_addr.clone(),
        &ExecuteMsg::Vote {
            proposal_id: 2,
            vote: Vote::Yes,
        },
        &[],
    );
    assert!(addr3_votes_yes_on_reset.is_ok());

    // now it's time to execute the Reset proposal
    assert!(app
        .execute_contract(
            addr1.clone(),
            multisig_addr.clone(),
            &ExecuteMsg::Execute { proposal_id: 2},
            &[],
        )
        .is_ok());

    // the proposal has been executed properly, counter is reset to 0
    assert_eq!(get_count(app.wrap(), &counter_addr), 0);

}