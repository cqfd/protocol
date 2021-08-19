import React from 'react';
import useSWR from 'swr';
import useApi from '../../utils/ApiContext';
import useWallet from '../../utils/WalletContext';
import { ThreadAccount } from '../../api';
import { display } from '../../utils';
import { messagesFetch } from '../../api';
import { Wallet } from '@project-serum/anchor';
import { useRouter } from 'next/router';

type PropsType = {
  thread: ThreadAccount,
}

export default function MessagePreview({thread}: PropsType): JSX.Element {
  const {program} = useApi();
  const {wallet} = useWallet();
  const {data: messages} = useSWR(thread ? ['/messages', program, thread, 1] : null, messagesFetch);
  const otherMembers = thread.thread.members.filter(member => member.key.toString() !== wallet?.publicKey.toString());

  return (
    <div>
      <div className='text-gray-800 dark:text-white'>{display(otherMembers[0].key)}</div>
      {messages && messages?.length > 0 && (
        <div className='text-sm text-gray-600 dark:text-gray-400'>{messages[0].message.text}</div>
      )}
    </div>
  );
}